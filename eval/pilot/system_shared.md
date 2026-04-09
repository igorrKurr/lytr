# Shared instructions (pilot A/B — both arms)

You solve the task in the user message. The user includes the **task prompt** and a **Reference LIR pipeline** (the same `starter.lir` the other eval arm uses). Implement **the same semantics** in your output language.

**Execution contract (read the user message):**
- If it says **stdin is empty**, do **not** read stdin.
- If it gives a **JSON array line** for stdin, parse that single line as JSON (list of integers) when your arm needs input.

Do not wrap your answer in markdown fences. No explanatory prose before the program.
