# Bench mark results

- OS: macOS 14.3.1
- CPU: M2
- RAM: 16GB

Execute 3 times and take the average

+ Basic: Interpreter with no optimization
+ Optimize1: Interpreter optimized with add, move, and pre-compute jumps
+ Optimize2: Interpreter optimized some loop pattern with specific instruction like: Clear, AddTo, MoveUntil

| Version   | factor.bf | mandelbrot.bf |
|-----------|-----------|---------------|
| Basic     | 10.66s    | 32.42s        |
| Optimize1 | 2.89s     | 6.34s         |
| Optimize2 | 1.79s     | 4.83s         |