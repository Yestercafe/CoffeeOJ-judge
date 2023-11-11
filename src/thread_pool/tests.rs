#[cfg(test)]
mod test {
    use crate::judge::runner::RunnerJob;
    use crate::{judge::task::Task, thread_pool::thread_pool_builder::ThreadPoolBuilder};

    #[test]
    fn thread_pool_join() {
        let thread_pool = ThreadPoolBuilder::new().build();
        for _ in 0..50 {
            let task = Task::new(
                1,
                "assets/1",
                "cpp",
                "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << 2 * a; }",
            );
            thread_pool.send_task(task);
        }
        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 0);
    }

    #[test]
    fn test_1359() {
        let thread_pool = ThreadPoolBuilder::new().build();
        for _ in 0..1 {
            let task = Task::new(
                1,
                "assets/1359",
                "cpp",
                r#"#include <bits/stdc++.h>
                using i64 = long long;
                 
                template <typename T, typename Sum = T>
                class Dst {
                    std::vector<T> tree, lazy;
                    std::vector<T> *arr;
                    int n, root, n4, end;
                 
                    void pushdown(int cl, int cr, int p) {
                        if (cl != cr && lazy[p]) {
                            int cm = cl + (cr - cl) / 2;
                            lazy[p * 2] += lazy[p];
                            lazy[p * 2 + 1] += lazy[p];
                            tree[p * 2] += lazy[p] * (cm - cl + 1);
                            tree[p * 2 + 1] += lazy[p] * (cr - cm);
                            lazy[p] = 0;
                        }
                    }
                 
                    Sum range_sum(int l, int r, int cl, int cr, int p) {
                        if (l <= cl && cr <= r) return tree[p];
                        pushdown(cl, cr, p);
                        int m = cl + (cr - cl) / 2;
                        Sum sum = 0;
                        if (l <= m) sum += range_sum(l, r, cl, m, p * 2);
                        if (r > m) sum += range_sum(l, r, m + 1, cr, p * 2 + 1);
                        return sum;
                    }
                 
                    void range_add(int l, int r, T val, int cl, int cr, int p) {
                        if (l <= cl && cr <= r) {
                            lazy[p] += val;
                            tree[p] += (cr - cl + 1) * val;
                            return;
                        }
                        pushdown(cl, cr, p);
                        int m = cl + (cr - cl) / 2;
                        if (l <= m) range_add(l, r, val, cl, m, p * 2);
                        if (r > m) range_add(l, r, val, m + 1, cr, p * 2 + 1);
                        tree[p] = tree[p * 2] + tree[p * 2 + 1];
                    }
                 
                    void build(int s, int t, int p) {
                        if (s == t) {
                            tree[p] = (*arr)[s];
                            return;
                        }
                        int m = s + (t - s) / 2;
                        build(s, m, p * 2);
                        build(m + 1, t, p * 2 + 1);
                        tree[p] = tree[p * 2] + tree[p * 2 + 1];
                    }
                 
                public:
                    explicit Dst(std::size_t n) {
                        this->n = n;
                        n4 = n * 4;
                        tree = std::vector<T>(n4, 0);
                        lazy = std::vector<T>(n4, 0);
                        end = n - 1;
                        root = 1;
                    }
                 
                    explicit Dst(std::vector<T>& v) {
                        n = v.size();
                        n4 = n * 4;
                        tree = std::vector<T>(n4, 0);
                        lazy = std::vector<T>(n4, 0);
                        arr = &v;
                        end = n - 1;
                        root = 1;
                        build(0, end, 1);
                        arr = nullptr;
                    }
                 
                    void show(int p, int depth = 0) {
                        if (p > n4 || tree[p] == 0) return;
                        show(p * 2, depth + 1);
                        for (int i = 0; i < depth; ++i) std::putchar('\t');
                        std::printf("%d:%d\n", tree[p], lazy[p]);
                        show(p * 2 + 1, depth + 1);
                    }
                 
                    Sum range_sum(int l, int r) { return range_sum(l, r, 0, end, root); }
                 
                    void range_add(int l, int r, int val) { range_add(l, r, val, 0, end, root); }
                };
                 
                int main()
                {
                    std::ios::sync_with_stdio(false);
                    std::cin.tie(nullptr);
                 
                    int n, m;
                    std::cin >> n >> m;
                    Dst<int> dst(n + 1);
                 
                    int prev;
                    std::cin >> prev;
                    for (int i = 1; i < m; ++i) {
                        int curr;
                        std::cin >> curr;
                        if (prev == curr) continue;
                 
                        int a = std::min(prev, curr), b = std::max(prev, curr);
                        // 1 .. 3  => 1 2
                        dst.range_add(a, b - 1, +1);
                 
                        prev = curr;
                    }
                 
                    std::vector<std::array<int, 3>> fee(n);
                    for (int i = 1; i < n; ++i) {
                        std::cin >> fee[i][0] >> fee[i][1] >> fee[i][2];
                    }
                 
                    i64 ans = 0;
                    for (int i = 1; i < n; ++i) {
                        int cnt = dst.range_sum(i, i);
                        i64 f1 = 1LL * cnt * fee[i][0];
                        i64 f2 = 1LL * cnt * fee[i][1] + fee[i][2];
                        ans += std::min(f1, f2);
                    }
                    std::cout << ans << std::endl;
                 
                    return 0;
                }"#,
            );
            thread_pool.send_task(task);
        }
        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 0);
    }
    
    #[test]
    fn thread_pool_thread_panic() {
        let thread_pool = ThreadPoolBuilder::new().build();
        for i in 0..20 {
            if i % 4 == 0 {
                thread_pool.send_job(|| -> Vec<RunnerJob> { panic!() })
            } else {
                let task = Task::new(
                    1,
                    "assets/1",
                    "cpp",
                    "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << 2 * a; }",
                );
                thread_pool.send_task(task);
            }
        }

        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 0);
        assert_eq!(thread_pool.panic_thread_count(), 5);
    }
}
