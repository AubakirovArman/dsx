# DSX Code — DeepSeek-Powered Terminal Coding Agent

<p align="center">
  <img src="plan/shell_architecture_current.png" alt="DSX Shell Architecture" width="700"/>
</p>

DSX Code (`dsx`) is a next-generation terminal coding assistant written in Rust and powered by DeepSeek V4. It acts as an autonomous local agent runtime, handling files, running shell commands, managing persistent sessions in SQLite, and presenting a stunning Tron-style interactive TUI dashboard.

---

## 🌐 Select Your Language / Тілді таңдаңыз / Выберите язык / 选择您的语言

To read the comprehensive product features, installation instructions, subcommands, and hotkey guides, select your preferred language documentation below:

*   **[🇺🇸 English Documentation (docs/en)](docs/en/README.md)**
*   **[🇷🇺 Русская Документация (docs/ru)](docs/ru/README.md)**
*   **[🇰🇿 Қазақша Құжаттама (docs/kk)](docs/kk/README.md)**
*   **[🇨🇳 中文使用指南 (docs/zh)](docs/zh/README.md)**

---

## ⚡ Quick Start / Тез арада іске қосу / Быстрый старт / 快速开始

### 1. Set API Key
```bash
export DEEPSEEK_API_KEY="sk-..."
```

### 2. Global Installation
```bash
cargo install --path . --force
```

### 3. Launch TUI Cockpit
```bash
dsx
```

---

## 📄 License

This project is licensed under the **[MIT License](LICENSE)** — free, open, and permissive for all personal and corporate uses.
