import { IconButton, Tooltip, useTheme as useMuiTheme } from "@mui/material";
import { Brightness4, Brightness7 } from "@mui/icons-material";
import { useTheme } from "../theme/MUIThemeProvider";

export function ThemeToggle() {
    const { mode, toggleTheme } = useTheme();
    const muiTheme = useMuiTheme();

    return (
        <Tooltip title={mode === "dark" ? "切换到浅色模式" : "切换到深色模式"}>
            <IconButton
                onclick={toggleTheme}
                color="inherit"
                sx={{
                    bgcolor: "rgba(255, 255, 255, 0.08)",
                    "&:hover": {
                        bgcolor: "rgba(255, 255, 255, 0.12)",
                    },
                }}
            >
                {mode === "dark" ? (
                    <Brightness7 sx={{ color: muiTheme.palette.text.primary }} />
                ) : (
                    <Brightness4 sx={{ color: muiTheme.palette.text.primary }} />
                )}
            </IconButton>
        </Tooltip>
    );
}