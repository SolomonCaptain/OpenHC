import { SignIn, SignUp } from "@clerk/clerk-react";
import {
    Container,
    Paper,
    Typography,
    Tabs,
    Tab,
    Box,
    useTheme,
} from "@mui/material";
import { useState } from "react";
import { ThemeToggle } from "../components/ThemeToggle";

export function AuthPage() {
    const [tabValue, setTabValue] = useState(0);
    const theme = useTheme();

    const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
        setTabValue(newValue);
    };

    return (
        <Box sx={{ position: "relative", minHeight: "100vh" }}>
            {/* 右上角主题切换按钮 */}
            <Box
                sx={{
                    position: "absolute",
                    top: 16,
                    right: 16,
                }}
            >
                <ThemeToggle />
            </Box>

            <Container
                maxWidth="sm"
                sx={{
                    minHeight: "100vh",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    py: 4,
                }}
            >
                <Paper
                    elevation={3}
                    sx={{
                        p: 4,
                        width: "100%",
                        backgroundColor: theme.palette.background.paper,
                    }}
                >
                    <Typography variant="h4" align="center" gutterBottom sx={{ mb: 3 }}>
                        星河
                    </Typography>

                    <Typography
                        variant="body2"
                        align="center"
                        color="text.secondary"
                        sx={{ mb: 3 }}
                    >
                        请登录或注册以继续使用
                    </Typography>

                    <Tabs
                        value={tabValue}
                        onChange={handleTabChange}
                        centered
                        sx={{ mb: 3 }}
                    >
                        <Tab label="登录" />
                        <Tab label="注册" />
                    </Tabs>

                    <Box sx={{ mt: 2 }}>
                        {tabValue === 0 ? (
                            <SignIn
                                routing="hash"
                                appearance={{
                                    baseTheme: undefined,
                                    elements: {
                                        formButtonPrimary: {
                                            backgroundColor: theme.palette.primary.main,
                                            "&:hover": {
                                                backgroundColor: theme.palette.primary.dark,
                                            },
                                        },
                                        card: {
                                            backgroundColor: "transparent",
                                            boxShadow: "none",
                                        },
                                    },
                                }}
                            />
                        ) : (
                            <SignUp
                                routing="hash"
                                appearance={{
                                    baseTheme: undefined,
                                    elements: {
                                        formButtonPrimary: {
                                            backgroundColor: theme.palette.primary.main,
                                            "&:hover": {
                                                backgroundColor: theme.palette.primary.dark,
                                            },
                                        },
                                        card: {
                                            backgroundColor: "transparent",
                                            boxShadow: "none",
                                        },
                                    },
                                }}
                            />
                        )}
                    </Box>
                </Paper>
            </Container>
        </Box>
    );
}