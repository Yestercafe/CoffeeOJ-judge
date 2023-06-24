#include <iostream>
int main() {
  std::ios::sync_with_stdio(false);
  std::cin.tie(nullptr);
  int T;
  std::cin >> T;
  while (T--) {
    int a;
    std::cin >> a;
    std::cout << a * a << std::endl;
  }
  return 0;
}
