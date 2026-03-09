import { createTheme, ThemeOptions, PaletteOptions } from "@mui/material/styles";

// 共享的主题配置基础
const baseThemeOptions: Omit<ThemeOptions, "palette"> = {
    typography: {
        fontFamily: '"Roboto", "Helvetica", "Arial", sans-serif',
        h1: {
            fontSize: "2.5rem",
            fontWeight: 700,
        },
        h2: {
            fontSize: "2rem",
            fontWeight: 600,
        },
        h3: {
            fontSize: "1.75rem",
            fontWeight: 600,
        },
        h4: {
            fontSize: "1.5rem",
            fontWeight: 600,
        },
        h5: {
            fontSize: "1.25rem",
            fontWeight: 500,
        },
        h6: {
            fontSize: "1rem",
            fontWeight: 500,
        },
    },
    shape: {
        borderRadius: 8,
    },
    components: {
        MuiButton: {
            styleOverrides: {
                root: {
                    textTransform: "none",
                    fontWeight: 500,
                },
            },
        },
        MuiPaper: {
            styleOverrides: {
                root: {
                    backgroundImage: "none",
                },
            },
        },
        MuiCard: {
            styleOverrides: {
                root: {
                    backgroundImage: "none",
                },
            },
        },
        MuiAppBar: {
            styleOverrides: {
                root: {
                    backgroundImage: "none",
                },
            },
        },
        MuiDrawer: {
            styleOverrides: {
                paper: {
                    backgroundImage: "none",
                },
            },
        },
    },
};

// 深色主题调色板
const darkPalette: PaletteOptions = {
    mode: "dark",
    primary: {
        main: "#6366f1",
        light: "#818cf8",
        dark: "#4f46e5",
        contrastText: "#ffffff",
    },
    secondary: {
        main: "#ec4899",
        light: "#f472b6",
        dark: "#db2777",
        contrastText: "#ffffff",
    },
    background: {
        default: "#0f172a",
        paper: "#1e293b",
    },
    text: {
        primary: "#f8fafc",
        secondary: " #94a3b8",
    },
    divider: "#334155",
    error: {
        main: "#ef4444",
        light: "#f87171",
        dark: "#dc2626",
    },
    warning: {
        main: "#f59e0b",
        light: "#fbbf24",
        dark: "#d97706",
    },
    info: {
        main: "#3b82f6",
        light: "#60a5fa",
        dark: "#2563eb",
    },
    success: {
        main: "#22c55e",
        light: "#4ade80",
        dark: "#16a34a",
    },
};

// 浅色主题调色板
const lightPalette: PaletteOptions = {
    mode: "light",
    primary: {
        main: "#6366f1",
        light: "#818cf8",
        dark: "#4f46e5",
        contrastText: "#ffffff",
    },
    secondary: {
        main: "#ec4899",
        light: "#f472b6",
        dark: "#db2777",
        contrastText: "#ffffff",
    },
    background: {
        default: "#f1f5f9",
        paper: "#ffffff",
    },
    text: {
        primary: "#1e293b",
        secondary: "#64748b",
    },
    divider: "#e2e8f0",
    error: {
        main: "#ef4444",
        light: "#fca5a5",
        dark: "#dc2626",
    },
    warning: {
        main: "#f59e0b",
        light: "#fcd34d",
        dark: "#d97706",
    },
    info: {
        main: "#3b82f6",
        light: "#93c5fd",
        dark: "#2563eb",
    },
    success: {
        main: "#22c55e",
        light: "#86efac",
        dark: "#16a34a",
    },
};

// 创建深色主题
export const darkTheme = createTheme({
  ...baseThemeOptions,
  palette: darkPalette,
});

// 创建浅色主题
export const lightTheme = createTheme({
  ...baseThemeOptions,
  palette: lightPalette,
});

// 主题类型
export type AppTheme = "light" | "dark";

// 导出主题映射
export const themes = {
  light: lightTheme,
  dark: darkTheme,
} as const;