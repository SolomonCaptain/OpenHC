import React, { useState, useEffect } from 'react';
import {
    Box,
    Button,
    Card,
    CardContent,
    CircularProgress,
    Typography,
    Chip,
    Alert,
    Paper,
} from "@mui/material";
import { apiService, type HelloResponse } from "../services/api";

const HelloWorld: React.FC = () => {
    const [helloData, setHelloData] = useState<HelloResponse | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [serviceStatus, setServiceStatus] = useState<{
        isRunning: boolean;
        cppAvailable: boolean;
    } | null>(null);

    // 检查服务状态
    useEffect(() => {
        const checkStatus = async () => {
            const status = await apiService.checkServiceStatus();
            setServiceStatus(status);
        };
        checkStatus();
    }, []);

    const fetchHelloWorld = async () => {
        setLoading(true);
        setError(null);
        try {
            const data = await apiService.getHello();
            setHelloData(data);
        } catch (err: any) {
            setError(err.message || "未能获取消息");
        } finally {
            setLoading(false);
        }
    };

    const getSourceColor = (source: string) => {
        return source.includes('C++') ? 'success' : 'info';
    };

    return (
        <Box
            sx={{
                minHeight: '100vh',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                padding: 3,
                backgroundColor: '#f5f5f5',
            }}
        >
            <Paper
                elevation={3}
                sx={{
                    maxWidth: 600,
                    width: '100%',
                    padding: 4,
                    borderRadius: 2,
                }}
            >
                <Typography variant="h3" component="h1" gutterBottom align="center">
                    FastAPI + C++ + React
                </Typography>

                <Typography variant="h6" color="text.secondary" align="center" gutterBottom>
                    Full-stack Hello World Application
                </Typography>

                {serviceStatus && (
                    <Alert
                        severity={serviceStatus.isRunning ? "success" : "error"}
                        sx={{ mb: 3 }}
                    >
                        Backend Status: {serviceStatus.isRunning ? "Running" : "Not Running"}
                        {serviceStatus.isRunning && (
                            <Chip
                                label={`C++ Library: ${serviceStatus.cppAvailable ? "Available" : "Not Available"}`}
                                color={serviceStatus.cppAvailable ? "success" : "warning"}
                                size="small"
                                sx={{ ml: 2 }}
                            />
                        )}
                    </Alert>
                )}

                <Box sx={{ display: 'flex', justifyContent: 'center', mb: 3 }}>
                    <Button
                        variant="contained"
                        size="large"
                        onClick={fetchHelloWorld}
                        disabled={loading}
                        startIcon={loading ? <CircularProgress size={20} /> : null}
                    >
                        {loading ? "Loading..." : "Get Message"}
                    </Button>
                </Box>

                {error && (
                    <Alert severity="error" sx={{ mb: 3 }}>
                        {error}
                    </Alert>
                )}

                {helloData && (
                    <Card variant="outlined">
                        <CardContent>
                            <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
                                <Typography variant="h6" component="h2">
                                    Message Received
                                </Typography>
                                <Chip
                                    label={helloData.source}
                                    color={getSourceColor(helloData.source)}
                                    size="small"
                                />
                            </Box>
                            <Typography
                                variant="h4"
                                component="div"
                                sx={{
                                    color: 'primary.main',
                                    fontWeight: 'bold',
                                    textAlign: 'center',
                                    py: 2,
                                }}
                            >
                                {helloData.message}
                            </Typography>
                            <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
                                This message was generated by {helloData.source.includes('C++') ? 'a C++ dynamic library' : 'Python fallback'} and served through FastAPI to your React frontend.
                            </Typography>
                        </CardContent>
                    </Card>
                )}

                <Box sx={{ mt: 4, textAlign: 'center' }}>
                    <Typography variant="body2" color="text.secondary">
                        Stack: React + TypeScript + Material-UI + FastAPI + C++
                    </Typography>
                </Box>
            </Paper>
        </Box>
    );
};

export default HelloWorld;