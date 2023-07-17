# CoffeeOJ-judge

## Build & Run

Need cargo and node.js.

Rust dev env and cargo:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Node.js and npm: <https://nodejs.org/>

Need tmux, please install by using instructions like `sudo apt install tmux` or `brew install tmux`.

```bash
./startup.sh
```

Print beautiful log with bunyan:

```
npm install bunyan
```

Then run this project with bunyan:

```
cargo run | ./node_modules/bunyan/bin/bunyan
```

## API

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
