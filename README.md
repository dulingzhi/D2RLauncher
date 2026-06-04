# D2R CN 多开启动器

<div align="center">

![D2R CN Launcher](https://img.shields.io/badge/D2R-CN%20Launcher-blue?style=for-the-badge)
![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8?style=for-the-badge&logo=tauri)
![Vue](https://img.shields.io/badge/Vue-3.x-4FC08D?style=for-the-badge&logo=vue.js)
![Rust](https://img.shields.io/badge/Rust-1.75+-CE422B?style=for-the-badge&logo=rust)

暗黑破坏神II：重制版（国服）多开启动器，基于 Tauri 2.x + Vue 3 + Rust 开发

[功能特性](#功能特性) • [下载安装](#下载安装) • [使用说明](#使用说明) • [开发指南](#开发指南)

</div>

---

## 📋 功能特性

### 核心功能
- ✅ **多开管理** - 支持同时运行多个游戏客户端
- 🔐 **Token 自动化** - 内置浏览器自动获取登录 Token，无需手动抓包
- 🎮 **智能监控** - 实时监控游戏进程状态和运行时长
- ⚡ **登录检测** - 自动检测登录完成，避免批量启动时 Token 冲突
- 🔄 **队列启动** - 批量启动多个账号，自动排队依次启动
- 🛠️ **内置 Handle** - 自动处理互斥锁，无需手动配置 Handle64.exe

### 界面特性
- 🎨 **现代化 UI** - 暗色主题，简洁美观
- 📊 **状态可视化** - 实时显示运行状态、游戏时长、进程 PID
- ⚙️ **灵活配置** - 支持游戏路径选择、启动延迟、登录超时等设置
- 💾 **数据持久化** - 账号信息和配置本地保存
- 🔔 **等待提示** - 启动时显示半透明遮罩，告知用户等待状态

### 技术特性
- 🚀 **轻量快速** - 基于 Rust 后端，性能优异
- 📦 **单文件发布** - 打包为单个 exe 文件，无需安装
- 🔄 **自动更新** - 内置 Tauri Updater，支持自动检测和更新
- 🛡️ **安全可靠** - Token 使用 Windows DPAPI 加密存储
- 🪟 **Windows API** - 直接调用系统 API，精准控制窗口和进程

---

## 📥 下载安装

### 方式一：下载发布版本（推荐）
1. 前往 [Releases](https://github.com/dulingzhi/D2RLauncher/releases) 页面
2. 下载最新版本的 `.exe` 安装包
3. 运行安装程序，按提示完成安装
4. 首次运行需要设置游戏路径（D2R.exe 所在目录）

### 方式二：从源码构建
参见 [开发指南](#开发指南) 章节

---

## 📖 使用说明

### 首次配置
1. **设置游戏路径**
   - 点击 `⚙️ 设置` 标签页
   - 点击 `📁 浏览` 按钮选择 `D2R.exe` 文件
   - 程序会自动提取目录路径
   - 点击 `保存设置`

2. **添加账号**
   - 返回 `账号管理` 标签页
   - 点击 `+ 添加账号` 按钮
   - 填写账号信息（标签、邮箱、大区）
   - 点击 `保存`

3. **获取 Token**
   - 在账号卡片上点击 `🔑 获取 Token` 按钮
   - 等待登录窗口打开
   - 输入账号密码登录战网
   - 登录成功后 Token 自动保存

### 启动游戏
#### 单个启动
- 点击账号卡片上的 `🚀 启动` 按钮
- 如果启用了登录检测，会显示等待遮罩
- 登录完成后遮罩自动消失

#### 批量启动
- 点击右上角的 `🚀 全部启动` 按钮
- 所有已配置 Token 的账号会依次启动
- 启用登录检测时会自动排队，确保前一个账号登录完成后再启动下一个

### 管理游戏
- **查看状态** - 账号卡片实时显示运行状态、时长、PID
- **关闭进程** - 游戏运行时，启动按钮变为 `⛔ 关闭进程`
- **刷新 Token** - Token 过期时点击 `🔄 刷新 Token` 重新获取

### 高级设置
- **登录检测** - 启用后监控注册表 Token 变化，确保登录完成
- **登录超时** - 设置等待登录完成的最长时间（默认 60 秒）
- **启动间隔** - 禁用登录检测时的批量启动间隔（默认 5 秒）

---

## 🛠️ 开发指南

### 环境要求
- **Node.js** 18.x 或更高（推荐使用 Bun）
- **Rust** 1.75 或更高
- **Bun** 最新版本
- **Windows 10/11** 64位

### 开发依赖
```bash
# 安装 Rust（如果尚未安装）
# 访问 https://rustup.rs/ 下载安装

# 安装 Bun（如果尚未安装）
# 访问 https://bun.sh/ 下载安装

# 克隆项目
git clone https://github.com/dulingzhi/D2RLauncher.git
cd d2r-launcher

# 安装前端依赖
bun install

# 安装 Rust 依赖（自动）
cd src-tauri
cargo build
```

### 开发命令
```bash
# 启动开发服务器（热重载）
bun run tauri dev

# 构建生产版本
bun run tauri build

# 前端类型检查
bun run build

# Rust 代码检查
cd src-tauri
cargo check

# Rust 测试
cargo test
```

### 项目结构
```
d2r-launcher/
├── src/                      # Vue 前端代码
│   ├── App.vue              # 主应用组件
│   ├── components/          # UI 组件
│   │   ├── AccountCard.vue  # 账号卡片
│   │   ├── AccountModal.vue # 账号编辑弹窗
│   │   ├── SettingsPanel.vue# 设置面板
│   │   └── LoginDialog.vue  # 登录对话框
│   ├── types.ts             # TypeScript 类型定义
│   └── main.ts              # 前端入口
├── src-tauri/               # Rust 后端代码
│   ├── src/
│   │   ├── lib.rs           # 主模块
│   │   ├── accounts.rs      # 账号管理
│   │   ├── token.rs         # Token 处理
│   │   ├── token_browser.rs # 浏览器登录
│   │   ├── token_monitor.rs # 登录检测
│   │   ├── launcher.rs      # 游戏启动
│   │   ├── game_monitor.rs  # 进程监控
│   │   ├── settings.rs      # 配置管理
│   │   └── embedded_resources.rs # 内置资源
│   ├── resources/           # 内置资源文件
│   │   └── handle64.exe     # Handle 工具
│   ├── Cargo.toml           # Rust 依赖配置
│   └── tauri.conf.json      # Tauri 配置
├── .github/workflows/       # CI/CD 配置
│   └── release.yml          # 发布工作流
├── package.json             # 前端依赖配置
└── README.md                # 本文件
```

### 核心技术栈
- **前端**
  - Vue 3 - 渐进式 JavaScript 框架
  - TypeScript - 类型安全
  - Vite - 快速构建工具
  - Bun - JavaScript 运行时

- **后端**
  - Rust - 系统级编程语言
  - Tauri - 跨平台桌面应用框架
  - Windows API - 进程和窗口控制
  - sysinfo - 进程监控
  - winreg - 注册表访问

### 关键实现
#### 1. Token 获取
使用 Tauri 创建独立的浏览器窗口，导航到战网登录页面，通过监听页面导航事件捕获 Token。

#### 2. 多开实现
使用 Handle64.exe 关闭游戏的互斥锁句柄，允许启动多个实例。Handle64.exe 在编译时嵌入到程序中，运行时自动释放到临时目录。

#### 3. 登录检测
监控注册表路径 `HKCU\SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI\WEB_TOKEN`，检测 Token 变化判断登录完成。

#### 4. 进程监控
后台线程每 2 秒扫描一次系统进程，匹配 D2R.exe 进程和窗口标题，更新运行状态和时长。

#### 5. 窗口重命名
启动游戏后使用 Windows API 枚举窗口，根据进程 PID 匹配窗口，将标题设置为 `[账号ID] 账号名 (大区)`，便于识别。

---

## 🔐 安全说明

- **Token 存储** - 使用 Windows DPAPI 加密存储，只有当前用户可解密
- **数据本地化** - 所有数据保存在本地，不上传到任何服务器
- **开源透明** - 源代码完全开放，可自行审查和编译
- **Handle64.exe** - 来自 Microsoft Sysinternals，用于进程句柄管理

---

## 📝 配置 CI/CD 和更新

### 设置 GitHub Actions
1. **生成更新签名密钥**
   ```bash
   cd src-tauri
   cargo tauri signer generate -w ~/.tauri/myapp.key
   ```

2. **配置 GitHub Secrets**
   - 进入仓库 Settings > Secrets and variables > Actions
   - 添加以下 Secrets：
     - `TAURI_SIGNING_PRIVATE_KEY` - 私钥内容
     - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - 私钥密码（如果有）

3. **更新 tauri.conf.json**
   - 将 `YOUR_UPDATER_PUBLIC_KEY` 替换为生成的公钥
   - 将 GitHub 仓库 URL 替换为实际地址

### 发布新版本
1. **更新版本号**
   ```bash
   # 编辑 src-tauri/tauri.conf.json，修改 version 字段
   # 例如: "version": "0.2.0"
   ```

2. **提交代码并打标签**
   ```bash
   git add .
   git commit -m "chore: bump version to 0.2.0"
   git tag v0.2.0
   git push origin main --tags
   ```

3. **自动构建流程**
   - GitHub Actions 自动触发构建
   - 自动生成更新日志（基于 git commit 历史）
   - 使用 UPX 压缩 exe（从 17MB 压缩到 4MB）
   - 生成 `latest.json` 文件供 Tauri Updater 使用
   - 创建 Draft Release，包含：
     - `d2r-launcher.exe` - 压缩后的可执行文件
     - `latest.json` - 更新配置文件
     - 自动生成的更新日志

4. **发布 Release**
   - 前往 GitHub Releases 页面
   - 检查自动生成的更新日志
   - 根据需要编辑说明
   - 点击 "Publish release"

### 更新日志规范
为了更好地自动生成更新日志，建议遵循以下 commit message 规范：
- `feat:` - 新功能
- `fix:` - Bug 修复
- `perf:` - 性能优化
- `docs:` - 文档更新
- `style:` - 代码格式调整
- `refactor:` - 代码重构
- `test:` - 测试相关
- `chore:` - 构建/工具配置

示例：
```bash
git commit -m "feat: 添加批量导入账号功能"
git commit -m "fix: 修复登录超时导致程序卡死的问题"
git commit -m "perf: 优化进程监控性能,降低 CPU 占用"
```

### Tauri Updater 工作流程
1. **应用启动** - 自动检查更新（后台进行）
2. **发现新版本** - 弹出对话框询问是否更新
3. **下载更新** - 从 GitHub Release 下载新版本
4. **验证签名** - 使用公钥验证文件完整性
5. **安装更新** - 替换旧版本，重启应用

用户无需手动下载，应用会自动处理整个更新流程。

---

## ❓ 常见问题

### Q: 程序无法启动游戏？
A: 检查设置中的游戏路径是否正确，确保指向 D2R.exe 所在目录。

### Q: Token 获取失败？
A: 确保网络连接正常，能访问战网登录页面。如果持续失败，可尝试使用 PowerShell 脚本手动获取 Token。

### Q: 多开时第二个客户端无法启动？
A: Handle64.exe 嵌入式释放可能失败，查看控制台日志确认是否正常释放到临时目录。

### Q: 登录检测一直等待？
A: 检查注册表路径权限，或在设置中禁用登录检测，使用固定延迟启动。

---

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

---

## 📄 开源协议

本项目采用 MIT 协议开源 - 详见 [LICENSE](LICENSE) 文件

---

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 前端框架
- [Rust](https://www.rust-lang.org/) - 系统编程语言
- [Handle (Sysinternals)](https://docs.microsoft.com/en-us/sysinternals/downloads/handle) - 进程句柄管理工具
- 原 PowerShell 脚本作者

---

<div align="center">

**⭐ 如果这个项目对你有帮助，请给个 Star！⭐**

Made with ❤️ by D2R Players

</div>

