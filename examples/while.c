int main() {
  int x = 0;
  int sum = 0;

  while (x < 10) {
    sum = sum + x;
    x = x + 1;
  }

  if (sum == 45) {
    return 3;
  }
  return 99;
}
