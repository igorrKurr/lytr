"""
Thesis metrics for Lytr: whether “LLM + LIR” is the efficient choice vs Python on the pilot.

Structured around three evidence pillars (see ``build_thesis_metrics`` return value ``pillars``).
"""
from __future__ import annotations

from typing import Any


THESIS_ONE_LINE = (
    "At equal accuracy on the pilot tasks, LIR should consume less LLM budget than Python "
    "(lower marginal cost after instruction, higher success per token, dominance on completion + repair)."
)


def build_thesis_metrics(
    accuracy: dict[str, Any],
    tot_lir: dict[str, int],
    tot_py: dict[str, int],
    *,
    reliability_lir: dict[str, Any],
) -> dict[str, Any]:
    """
    Returns ``thesis_metrics`` for ``results_pilot_comparison_<run_id>.json``.

    **Pillar 1 — marginal after instruction:** user-message (tiktoken) + API completion,
    per task — excludes *fixed* system text so you can reason about cost once the instruction
    budget is stable. The fair pilot still uses *different* system prompts per arm; see
    ``pillars["1_marginal_after_instruction"]["harness_note"]``.

    **Pillar 2 — success per spend:** passes per 1000 total API tokens; passes per 1000
    marginal tokens; LIR single-shot (no ``lir check`` retry).

    **Pillar 3 — completion + repair:** completion tokens per task (verbose output); LIR
    retry flags extra rounds (cost is folded into API totals, not split per call here).
    """
    n = int(accuracy.get("tasks") or 0)
    n_safe = max(n, 1)

    lir_pass = int(accuracy.get("lir_pass_count") or 0)
    py_pass = int(accuracy.get("python_pass_count") or 0)

    def _arm(tot: dict[str, int], pass_count: int) -> dict[str, float]:
        api_p = int(tot.get("tokens_prompt", 0) or 0)
        api_c = int(tot.get("tokens_completion", 0) or 0)
        sys_c = int(tot.get("tokens_prompt_system", 0) or 0)
        usr_c = int(tot.get("tokens_prompt_user", 0) or 0)
        api_total = api_p + api_c
        marginal = usr_c + api_c
        return {
            "api_prompt_tokens": float(api_p),
            "api_completion_tokens": float(api_c),
            "api_total_tokens": float(api_total),
            "tiktoken_system_content_tokens": float(sys_c),
            "tiktoken_user_content_tokens": float(usr_c),
            "marginal_proxy_tokens_total": float(marginal),
            "api_total_tokens_per_task_avg": round(api_total / n_safe, 4),
            "marginal_proxy_tokens_per_task_avg": round(marginal / n_safe, 4),
            "api_completion_tokens_per_task_avg": round(api_c / n_safe, 4),
            "tiktoken_user_content_tokens_per_task_avg": round(usr_c / n_safe, 4),
            "tiktoken_system_content_tokens_per_task_avg": round(sys_c / n_safe, 4),
            "successes_per_1000_api_total_tokens": round(
                pass_count * 1000 / max(api_total, 1), 6
            ),
            "successes_per_1000_marginal_proxy_tokens": round(
                pass_count * 1000 / max(marginal, 1), 6
            ),
        }

    arm_lir = _arm(tot_lir, lir_pass)
    arm_py = _arm(tot_py, py_pass)

    ml = arm_lir["marginal_proxy_tokens_per_task_avg"]
    mp = arm_py["marginal_proxy_tokens_per_task_avg"]
    tl = arm_lir["api_total_tokens_per_task_avg"]
    tp = arm_py["api_total_tokens_per_task_avg"]
    cl = arm_lir["api_completion_tokens_per_task_avg"]
    cp = arm_py["api_completion_tokens_per_task_avg"]

    ratio_marg_py_div_lir = round(mp / ml, 6) if ml > 0 else None
    ratio_total_py_div_lir = round(tp / tl, 6) if tl > 0 else None
    ratio_comp_py_div_lir = round(cp / cl, 6) if cl > 0 else None

    accuracy_parity = n > 0 and lir_pass == py_pass == n

    lir_win_marginal = ml < mp if ml or mp else False
    lir_win_total = tl < tp if tl or tp else False
    lir_win_completion = cl < cp if cl or cp else False

    spl_api = arm_lir["successes_per_1000_api_total_tokens"]
    spp_api = arm_py["successes_per_1000_api_total_tokens"]
    lir_win_success_per_api = spl_api > spp_api

    spl_marg = arm_lir["successes_per_1000_marginal_proxy_tokens"]
    spp_marg = arm_py["successes_per_1000_marginal_proxy_tokens"]
    lir_win_success_per_marginal = spl_marg > spp_marg

    retry_any = bool(reliability_lir.get("retry_used_any_task"))
    lir_single_shot_all_tasks = accuracy_parity and not retry_any

    # Legacy composite verdicts (kept for scripts)
    thesis_strict = accuracy_parity and lir_win_marginal and lir_win_total
    thesis_lenient = accuracy_parity and (lir_win_marginal or lir_win_total)
    thesis_output_only = accuracy_parity and lir_win_completion and not thesis_lenient

    if not accuracy_parity:
        overall = "blocked_accuracy_mismatch"
    elif thesis_strict:
        overall = "supported_strict"
    elif thesis_lenient:
        overall = "supported_lenient"
    elif thesis_output_only:
        overall = "supported_completion_only"
    else:
        overall = "not_supported"

    pillars: dict[str, Any] = {
        "1_marginal_after_instruction": {
            "intent": (
                "Lower marginal tokens per task once instruction budget is held constant or "
                "minimized (e.g. production system prompt)."
            ),
            "marginal_proxy_per_task_avg": {
                "lir": ml,
                "python": mp,
                "ratio_python_div_lir": ratio_marg_py_div_lir,
            },
            "system_instruction_tokens_per_task_avg": {
                "lir": arm_lir["tiktoken_system_content_tokens_per_task_avg"],
                "python": arm_py["tiktoken_system_content_tokens_per_task_avg"],
            },
            "lir_lower_marginal_than_python": lir_win_marginal,
            "harness_note": (
                "Fair pilot uses different system prompts per arm (LIR dialect help vs short Python). "
                "This is not a constant-instruction experiment; compare marginal to isolate task+user "
                "cost, and plan a separate run with matched or minimized system text for production."
            ),
        },
        "2_success_per_spend_and_repair": {
            "intent": (
                "Higher success per token (pass rate vs spend) on the same task set; account for "
                "retries / repair rounds as extra LLM usage."
            ),
            "successes_per_1000_api_total_tokens": {
                "lir": spl_api,
                "python": spp_api,
                "lir_higher": lir_win_success_per_api,
            },
            "successes_per_1000_marginal_proxy_tokens": {
                "lir": spl_marg,
                "python": spp_marg,
                "lir_higher": lir_win_success_per_marginal,
            },
            "lir_arm": {
                "retry_used_any_task": retry_any,
                "all_tasks_passed_without_lir_retry": lir_single_shot_all_tasks,
            },
            "note": (
                "Retry rounds add an extra API call (prompt+completion in totals). "
                "Success-per-token is lower when retries fire even if the task eventually passes."
            ),
        },
        "3_completion_and_repair": {
            "intent": (
                "Dominance on completion (and repair cost), not only raw prompt size — verbose "
                "languages hurt most in model output length and fix-up rounds."
            ),
            "completion_tokens_per_task_avg": {"lir": cl, "python": cp},
            "ratio_python_div_lir_completion_per_task": ratio_comp_py_div_lir,
            "lir_lower_completion_than_python": lir_win_completion,
            "repair_signal": {
                "lir_retry_used": retry_any,
                "python_retry": False,
                "note": "LIR `lir check` retries are the repair mechanism tracked here.",
            },
        },
    }

    # How many pillars show LIR ahead on the headline boolean for that pillar
    pillar_scores = {
        "1_marginal": lir_win_marginal,
        "2_success_density": lir_win_success_per_api or lir_win_success_per_marginal,
        "2_no_repair_penalty": lir_single_shot_all_tasks,
        "3_completion": lir_win_completion,
    }

    return {
        "schema_version": 2,
        "thesis_statement": THESIS_ONE_LINE,
        "pillars": pillars,
        "pillar_summary": {
            "evidence_1_marginal_after_instruction_lir_wins": lir_win_marginal,
            "evidence_2_success_per_token_lir_higher_api": lir_win_success_per_api,
            "evidence_2_success_per_token_lir_higher_marginal": lir_win_success_per_marginal,
            "evidence_2_no_lir_retry_rounds": lir_single_shot_all_tasks,
            "evidence_3_completion_lir_wins": lir_win_completion,
        },
        "definitions": {
            "marginal_proxy": (
                "tiktoken(user message bodies) + API completion tokens — excludes system text."
            ),
            "successes_per_1000_marginal_proxy_tokens": (
                "pass_count * 1000 / marginal_proxy_tokens_total — success per unit of "
                "task+output token cost (instruction body excluded)."
            ),
        },
        "accuracy_gate": {
            "parity_full_pass_both_arms": accuracy_parity,
            "tasks": n,
        },
        "per_arm": {"lir": arm_lir, "python": arm_py},
        "head_to_head": {
            "ratio_python_div_lir_marginal_proxy_per_task": ratio_marg_py_div_lir,
            "ratio_python_div_lir_total_api_per_task": ratio_total_py_div_lir,
            "ratio_python_div_lir_completion_per_task": ratio_comp_py_div_lir,
            "lir_lower_marginal_proxy_per_task": lir_win_marginal,
            "lir_lower_total_api_per_task": lir_win_total,
            "lir_lower_completion_per_task": lir_win_completion,
            "lir_higher_successes_per_1000_api_tokens": lir_win_success_per_api,
            "lir_higher_successes_per_1000_marginal_proxy": lir_win_success_per_marginal,
            "interpretation_ratios": (
                "For ratio_python_div_lir_* : values >1.0 mean Python spends more than LIR "
                "on that denominator."
            ),
        },
        "reliability": {
            "lir_retry_used": retry_any,
            "note": "Repair cost is inside API totals when retries occur.",
        },
        "verdict": {
            "overall": overall,
            "lir_wins_marginal_and_total_strict": thesis_strict,
            "lir_wins_marginal_or_total_lenient": thesis_lenient,
            "lir_wins_completion_compactness": accuracy_parity and lir_win_completion,
            "pillar_flags": pillar_scores,
        },
    }
