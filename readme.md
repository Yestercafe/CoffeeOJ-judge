# CoffeeOJ-judger

POST 127.0.0.1:4514/api/v1/submit:

```
{
	"source": "#include <iostream>\nint main() {\nstd::ios::sync_with_stdio(false);\nstd::cin.tie(nullptr);\nint T;\nstd::cin >> T;\nwhile (T--) {\nint a;\nstd::cin >> a;\nstd::cout << a * a << std::endl;\n}\nreturn 0;\n}\n",
	"lang": "cpp",
	"problem_id": "p1001"
}
```

Response:

```
200 OK
body: Ok(())
```
