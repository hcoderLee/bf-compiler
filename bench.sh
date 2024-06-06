run_program() {
    echo "Running $1"
    TIME_OUTPUT=$({ time -p target/release/bf-compiler programs/$1 > /dev/null; } 2>&1)
    REAL_TIME=$(echo $TIME_OUTPUT | grep real | awk '{print $2}')
    echo "$1 took $REAL_TIME seconds to run"
}

# Calculate the time taken to run the factor.bf program
echo 179424691 | run_program "factor.bf"
# Calculate the time taken to run the mandelbrot.bf program
run_program "mandelbrot.bf"
