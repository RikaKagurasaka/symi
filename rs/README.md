如果你在 Rust 环境下开发，以下工具会让你更方便：

A. tree-sitter (Rust crate)
这是官方的 Rust 绑定。

作用：在 Rust 中加载生成的 C 解析器。
提示：由于 Tree-sitter 生成的是 C 代码，你需要通过这个 crate 将其链接到你的 Rust 项目中。

B. topiary (由 Tweag 开发)
作用：这是一个基于 Tree-sitter 的通用格式化引擎。
为什么方便：它已经帮你解决了“如何从 Tree-sitter 树重新生成字符串”的问题。你只需要编写一种特殊的 .scm 查询文件来定义格式化规则（比如：遇到这个节点就换行，遇到那个节点就缩进），它就能帮你完成格式化任务。

C. tree-sitter-graph
作用：如果你需要将代码逻辑可视化（比如生成数据流图、控制流图），这个库可以将解析树转换为 Graphviz 的 DOT 格式。
