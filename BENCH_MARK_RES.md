# Bench mark results

- OS: macOS 14.3.1
- CPU: M2
- RAM: 16GB

Execute 3 times and take the average

+ Basic: Interpreter with no optimization
+ Optimize: Interpreter optimized with add, move, and pre-compute jumps

| Version  | factor.bf | mandelbrot.bf |
|----------|-----------|---------------|
| Basic    | 10.66s    | 32.42s        |
| Optimize | 2.89s     | 6.34s         |
