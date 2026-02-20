import React, { useState } from 'react';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import { AppBar, Tab, Tabs, Box, Toolbar, Typography, Container } from '@mui/material';
import HelloWorld from './components/HelloWorld';
import FileManager from './components/FileManager';

const theme = createTheme({
    palette: {
        mode: 'light',
        primary: { main: '#1976b2' },
        secondary: { main: '#dc004e' },
    },
    typography: {
        fontFamily: '"Roboto", "Helvetica", "Arial", sans-serif',
    },
});

interface TabPanelProps {
    children?: React.ReactNode;
    index: number;
    value: number;
}

function TabPanel(props: TabPanelProps) {
    const { children, value, index, ...other } = props;
    return (
        <div
            role="tabpanel"
            hidden={value !== index}
            id={`simple-tabpanel-${index}`}
            aria-labelledby={`simple-tab-${index}`}
            {...other}
        >
            {value === index && <Box sx={{ p: 3 }}>{children}</Box>}
        </div>
    );
}

const App: React.FC = () => {
    const [tabValue, setTabValue] = useState(0);

    const handleTabChange = (event: React.SyntheticEvent, newValue: number) => {
        setTabValue(newValue);
    };

    return (
        <ThemeProvider theme={theme}>
            <CssBaseline />
            <AppBar position="static">
                <Toolbar>
                    <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
                        演示页
                    </Typography>
                    <Tabs value={tabValue} onChange={handleTabChange} textColor="inherit" indicatorColor="secondary">
                        <Tab label="测试" />
                        <Tab label="文件管理器" />
                    </Tabs>
                </Toolbar>
            </AppBar>
            <Container maxWidth="lg">
                <TabPanel value={tabValue} index={0}>
                    <HelloWorld />
                </TabPanel>
                <TabPanel value={tabValue} index={1}>
                    <FileManager />
                </TabPanel>
            </Container>
        </ThemeProvider>
    );
};

export default App;