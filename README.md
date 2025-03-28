# TabGlide – Seamless tab switching with your scroll wheel

## 📖 Overview

This AutoHotkey (AHK) script allows **seamless tab switching** in supported applications by **scrolling the mouse wheel** while hovering over the tab bar. It provides an **intuitive and efficient** way to navigate between open tabs in browsers and other multi-tab applications.

## 🌟 Features

- 🔄 **Effortless Tab Switching** – Use the scroll wheel to switch tabs quickly.
- 💻 **Works Across Multiple Applications** – Supports browsers (Chrome, Firefox, Edge, etc.), Windows Explorer, and more.
- ⚙️ **Customizable Settings** – Adjust return behavior, scroll sensitivity, and activation area.
- ⚡ **Lightweight & Fast** – Runs with minimal performance impact.
- 🛠 **Built-in Debug Mode** – Real-time GUI monitoring for troubleshooting.

## 📷 Demo

_(Add a demo GIF or screenshot here for clarity.)_

## 🔧 Installation

1. **Install AutoHotkey** (if not already installed) from [AutoHotkey.com](https://www.autohotkey.com/).
2. **Download the script** or clone this repository:
   git clone <https://github.com/yourusername/scroll-wheel-tab-switching.git>

3. **Run the script** by double-clicking `scroll-tabs.ahk`.
4. _(Optional)_ **Set it to run on startup** for seamless operation.

## ▶️ How to Use

- **Hover over the tab bar** in a supported application.
- **Scroll Up** → Move to the previous tab.
- **Scroll Down** → Move to the next tab.
- **Middle-click (optional)** → Open the debug GUI (if enabled in settings).

## ⚙️ Configuration

Modify the following global variables in `scroll-tabs.ahk`:

```ahk
global TOP_REGION_PIXEL_LIMIT := 50   ; Defines the activation area for scrolling.
global ENABLE_WINDOW_RETURN := true    ; Return to the original window after switching.
global RETURN_AFTER_MS := 700         ; Time before returning to the original window (in ms).
global DEBUG := true                  ; Enable debug GUI.
```

## ✅ Supported Applications

This script is compatible with the following applications:

- ✅ Chrome, Firefox, Edge, Opera, Brave
- ✅ Windows Explorer
- ✅ Windows Terminal

To customize compatibility, modify the `ALLOWED_PROGRAMS` list in the script.

## 🛠 Built With

- **AutoHotkey v2** – A powerful scripting language for Windows automation.

## 📝 Contributing

Contributions are welcome! To contribute:

1. **Fork the repository**.
2. **Create a new branch** (`feature/new-feature`).
3. **Submit a pull request** with your changes.

## 🐞 Troubleshooting & FAQ

**Q: The script is not working in my browser.**
A: Ensure that your browser’s process name is included in the `ALLOWED_PROGRAMS` list.

**Q: How can I adjust the scroll sensitivity?**
A: Modify the `TOP_REGION_PIXEL_LIMIT` value in the script settings.

## 📄 License

This project is licensed under the **GNU GENERAL PUBLIC LICENSE** – feel free to use and modify it!

## 🤝 Acknowledgments

Special thanks to the **AHK community** for their support and contributions.

---

Enjoy effortless tab navigation with **Scroll-Wheel Tab Switching**! 🚀
