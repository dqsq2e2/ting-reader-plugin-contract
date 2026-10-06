# Changelog

## 2.0.3 - 2026-10-06

- 新增 `bookmarks_read` 和 `bookmarks_write`，分别声明当前用户书签的读取和管理权限。
- 书签权限不接受用户身份或额外作用域字段，用户身份由宿主注入。
- 保持现有插件调用协议和 ABI 不变。

## 2.0.1 - 2026-09-29

- 最低 Rust 版本调整为 1.93，并修正相关实现以支持该工具链。
- 插件协议和最低主程序版本要求保持不变。

## 2.0.0 - 2026-09-29

- Published the standardized Ting Reader plugin contract.
- Added manifest, capability, format, scraper, resource, protocol, and Native
  ABI v2 declarations.
