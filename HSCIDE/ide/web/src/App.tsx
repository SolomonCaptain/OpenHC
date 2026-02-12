import React from "react";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import CssBaseline from "@mui/material/CssBaseline";
import HelloWorld from "./components/HelloWorld";

// 创建主题
const theme = createTheme({
    palette: {
        mode: 'light',
        primary: {
            main: '#1976b2',
        },
        secondary: {
            main: '#dc004e',
        },
    },
    typography: {
        fontFamily: '"Roboto", "Helvetica", "Arial", sans-serif',
    },
});

const App: React.FC = () => {
    return (
        <ThemeProvider theme={theme}>
            <CssBaseline />
            <HelloWorld />
        </ThemeProvider>
    );
};

export default App;