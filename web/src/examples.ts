// Example programs for the picker.

export const EXAMPLES: Record<string, string> = {
  fibonacci: `// First 10 Fibonacci numbers
let a = 0;
let b = 1;
let t = 0;
let n = 10;
while (n > 0) {
    print(a);
    t = a + b;
    a = b;
    b = t;
    n = n - 1;
}`,

  countdown: `// Countdown with a branch
let n = 5;
while (n >= 0) {
    if (n == 0) {
        print(999);
    } else {
        print(n);
    }
    n = n - 1;
}`,

  'sum 1..100': `// Gauss's trick, the slow way
let sum = 0;
let i = 1;
while (i <= 100) {
    sum = sum + i;
    i = i + 1;
}
print(sum);`,

  collatz: `// Collatz sequence from 27: print the step count
let n = 27;
let steps = 0;
while (n != 1) {
    if (n % 2 == 0) {
        n = n / 2;
    } else {
        n = 3 * n + 1;
    }
    steps = steps + 1;
}
print(steps);`,

  primes: `// Primes below 50 (trial division)
let n = 2;
let d = 0;
let prime = 0;
while (n < 50) {
    d = 2;
    prime = 1;
    while (d * d <= n) {
        if (n % d == 0) {
            prime = 0;
        }
        d = d + 1;
    }
    if (prime == 1) {
        print(n);
    }
    n = n + 1;
}`,
}
