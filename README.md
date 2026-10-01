# ting-reader-plugin-contract

Ting Reader 的公开插件协议契约。主程序后端与插件 SDK 使用这里的类型，统一插件清单、能力操作、请求响应、资源接口和 Native ABI。Rust crate 名称为 `ting-plugin-contract`。

## 包含哪些内容

| 模块 | 定义内容 |
| --- | --- |
| `manifest` | 插件清单、权限和最低版本要求 |
| `capability` | 九类能力声明、固定操作及能力注册类型 |
| `format` / `format_registry` | 扩展格式声明及通用匹配规则 |
| `format_calls` | 格式调用、元数据提取、资源引用及 staging 写回数据 |
| `scraper` | 元数据搜索请求、结果和校验规则 |
| `resources` | 资源句柄、资源操作及生命周期数据 |
| `protocol` | 调用结果信封、错误结构和错误码 |
| `native_abi` | Native 插件的 Rust ABI 声明，与 C 头文件对应 |

本 crate 只依赖通用类型库。数据库、HTTP 服务、文件访问、JavaScript / WASM 运行时和插件业务由主程序或 SDK 实现。

扩展格式插件在自己的清单中声明扩展名和已实现的操作，通过 SDK 完成格式识别、元数据提取及可选写回。格式解析和专属元数据规则由独立插件维护，主程序使用通用契约分发调用。

## 使用

### 在 Rust 插件中引用

Rust 插件在 `Cargo.toml` 中引用：

```toml
[dependencies]
ting-plugin-sdk = { git = "https://github.com/dqsq2e2/ting-plugin-sdk.git", tag = "v2.0.1" }
```

在插件的 `src/lib.rs` 中导入：

```rust
use ting_plugin_sdk::contract::protocol::PluginErrorCode;
use ting_plugin_sdk::contract::scraper::SearchRequest;
```

例如，处理元数据搜索输入：

```rust
use serde_json::Value;
use ting_plugin_sdk::{Result, SdkError};
use ting_plugin_sdk::contract::scraper::SearchRequest;

fn parse_search_request(input: Value) -> Result<SearchRequest> {
    let request: SearchRequest =
        serde_json::from_value(input).map_err(SdkError::parse)?;
    request.validate().map_err(SdkError::invalid)?;
    Ok(request)
}
```

JavaScript 插件使用项目根目录的 `sdk.mjs`，按能力文档中的输入输出字段实现业务；元数据结果可通过 `publishSearch(parsed, request)` 转换。

### 在独立 Rust 项目中引用

在项目的 `Cargo.toml` 添加固定 tag 依赖：

```toml
[dependencies]
ting-plugin-contract = { git = "https://github.com/dqsq2e2/ting-reader-plugin-contract.git", tag = "v2.0.1" }
serde_json = "1"
```

Rust 导入路径使用 `ting_plugin_contract`。以下 `src/main.rs` 将 JSON 转成契约类型并验证：

```rust
use serde_json::json;
use ting_plugin_contract::scraper::SearchRequest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request: SearchRequest = serde_json::from_value(json!({
        "title": "示例书籍",
        "author": null,
        "narrator": null,
        "page": 1,
        "page_size": 20,
        "filters": {},
    }))?;
    request.validate()?;
    println!("第 {} 页，每页 {} 条", request.page, request.page_size);
    Ok(())
}
```

运行 `cargo run`。Cargo 在首次构建时获取依赖，后续使用本地缓存；提交项目的 `Cargo.lock`，保留实际解析到的依赖版本。

### 其他语言的 Native 插件

使用 [ting_plugin_native.h](./include/ting_plugin_native.h) 中的 C ABI 声明，实现对应入口并按公开数据结构处理请求和结果。

## 版本与构建

- 当前 crate 版本为 `2.0.1`，发布 tag 为 `v2.0.1`。
- 使用 Rust 2024 edition，最低 Rust 版本为 `1.93`。
- `MIN_PLUGIN_CORE_VERSION` 当前为 `2.0.0`，清单的 `min_core_version` 应满足此要求。
- 插件自身的 `version` 独立维护，不需要与契约或主程序版本相同。
- 发布 tag 保持不可变。协议发生破坏性变更时发布新的主版本，并同步更新 SDK 和最低主程序版本要求。

在本仓库验证契约：

```sh
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## 许可证

AGPL-3.0-only，见 [LICENSE](./LICENSE)。
