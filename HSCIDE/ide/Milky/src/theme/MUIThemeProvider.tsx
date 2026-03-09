import { CssBaseline } from "@mui/material";
import { ReactNode } from "react";
import { ThemeProvider } from "./ThemeContext";

interface MUIThemeProviderProps {
    children: ReactNode;
}

export function MUIThemeProvider({ children }: MUIThemeProviderProps) {
    return (
        <ThemeProvider>
            <CssBaseline />
            {children}
        </ThemeProvider>
    );
}

// 导出主题相关的 Hook
export { useTheme, useThemeContext } from "./ThemeContext";
export type { AppTheme } from "./theme";