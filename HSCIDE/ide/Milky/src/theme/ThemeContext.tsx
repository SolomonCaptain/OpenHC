import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { ThemeProvider as MuiThemeProvider } from "@mui/material/styles";
import { themes, AppTheme } from "./theme";

// 主题上下文类型定义
interface ThemeContextType {
  mode: AppTheme;
  toggleTheme: () => void;
  setTheme: (mode: AppTheme) => void;
}

// 创建主题上下文
const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

// 本地存储键名
const THEME_STORAGE_KEY = "hsc-studio-theme";

// 从本地存储获取主题
function getStoredTheme(): AppTheme {
  if (typeof window === "undefined") return "dark";
  
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  
  // 检测系统主题偏好
  if (window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches) {
    return "light";
  }
  
  return "dark";
}

// 主题提供者属性
interface ThemeProviderProps {
  children: ReactNode;
}

// 主题提供者组件
export function ThemeProvider({ children }: ThemeProviderProps) {
  const [mode, setMode] = useState<AppTheme>(getStoredTheme);

  // 监听系统主题变化
  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    
    const handleChange = (e: MediaQueryListEvent) => {
      // 只有在没有手动设置主题时才跟随系统
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      if (!stored) {
        setMode(e.matches ? "dark" : "light");
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, []);

  // 持久化主题设置
  useEffect(() => {
    localStorage.setItem(THEME_STORAGE_KEY, mode);
  }, [mode]);

  // 切换主题
  const toggleTheme = () => {
    setMode((prevMode) => (prevMode === "light" ? "dark" : "light"));
  };

  // 设置指定主题
  const setTheme = (newMode: AppTheme) => {
    setMode(newMode);
  };

  // 上下文值
  const contextValue: ThemeContextType = {
    mode,
    toggleTheme,
    setTheme,
  };

  return (
    <ThemeContext.Provider value={contextValue}>
      <MuiThemeProvider theme={themes[mode]}>
        {children}
      </MuiThemeProvider>
    </ThemeContext.Provider>
  );
}

// 自定义 Hook：使用主题上下文
export function useThemeContext(): ThemeContextType {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error("useThemeContext must be used within a ThemeProvider");
  }
  return context;
}

// 导出便捷的 Hook
export { useThemeContext as useTheme };