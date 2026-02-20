import React, { useState, useEffect } from "react";
import {
    Box,
    Button,
    Card,
    CardContent,
    TextField,
    Typography,
    List,
    ListItem,
    ListItemText,
    ListItemSecondaryAction,
    IconButton,
    Paper,
    Alert,
    CircularProgress,
    Divider,
    Input,
    Snackbar,
} from "@mui/material";
import {
    Delete as DeleteIcon,
    Edit as EditIcon,
    Refresh as RefreshIcon,
    Upload as UploadIcon,
    Save as SaveIcon,
    Cancel as CancelIcon,
} from "@mui/icons-material";
import { apiService } from "../services/api";

const FileManager: React.FC = () => {
    const [files, setFiles] = useState<string[]>([]);
    const [selectedFile, setSelectedFile] = useState<string | null>(null);
    const [fileContent, setFileContent] = useState('');
    const [newFileName, setNewFileName] = useState('');
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [successMessage, setSuccessMessage] = useState<string | null>(null);
    const [uploading, setUploading] = useState(false);

    // 加载文件列表
    const loadFiles = async () => {
        setLoading(true);
        setError(null);
        try {
            const list = await apiService.listFiles();
            setFiles(list);
        } catch (err: any) {
            setError('未能获取文件列表：' + (err.message || "未知错误"));
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadFiles();
    }, []);

    // 读取文件内容
    const handleReadFile = async (filename: string) => {
        setLoading(true);
        setError(null);
        try {
            const data = await apiService.readFile(filename);
            setFileContent(data.content);
            setSelectedFile(filename);
        } catch (err: any) {
            setError('未能读取文件：' + (err.message || "未知错误"));
        } finally {
            setLoading(false);
        }
    };

    // 保存文件（新建或更新）
    const handleSaveFile = async () => {
        const filename = selectedFile || newFileName;
        if (!filename) {
            setError('请选择或输入文件名');
            return;
        }
        setLoading(true);
        setError(null);
        try {
            await apiService.saveFile(filename, fileContent);
            setSuccessMessage(`文件 "${filename}" 保存成功`);
            // 如果是新文件，刷新列表并重置新建状态
            if (!selectedFile) {
                setNewFileName('');
                setSelectedFile(filename);
                await loadFiles();
            } else {
                // 仅刷新列表
                await loadFiles();
            }
        } catch (err: any) {
            setError('未能保存文件：' + (err.message || "未知错误"))
        } finally {
            setLoading(false);
        }
    };

    // 删除文件
    const handleDeleteFile = async (filename: string) => {
        if (!window.confirm(`确定要删除文件 "${filename}" 吗？`)) {
            return;
        }
        setLoading(true);
        setError(null);
        try {
            await apiService.deleteFile(filename);
            setSuccessMessage(`文件 "${filename}" 删除成功`);
            if (selectedFile === filename) {
                setSelectedFile(null);
                setFileContent('');
            }
            await loadFiles();
        } catch (err: any) {
            setError('未能删除文件：' + (err.message || "未知错误"))
        } finally {
            setLoading(false);
        }
    };

    // 上传文件
    const handleUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
        const file = event.target.files?.[0];
        if (!file) {
            return;
        }
        setUploading(true);
        setError(null);
        try {
            const result = await apiService.uploadFile(file);
            setSuccessMessage(`文件 "${result.filename}" 上传成功`);
            await loadFiles();
        } catch (err: any) {
            setError('未能上传文件：' + (err.message || "未知错误"))
        } finally {
            setUploading(false);
            // 清空上传文件，允许上传同一文件
            event.target.value = '';
        }
    };

    // 取消编辑
    const handleCancel = () => {
        setSelectedFile(null);
        setFileContent('');
        setNewFileName('');
    };

    return (
        <Box sx={{ p: 3 }}>
            <Typography variant="h4" gutterBottom>
                文件管理器
            </Typography>

            {/* 错误提示 */}
            {error && (
                <Alert severity="error" sx={{ mb: 2 }} onClose={() => setError(null)}>
                    {error}
                </Alert>
            )}

            {/* 成功提示 */}
            <Snackbar
                open={!!successMessage}
                autoHideDuration={3000}
                onClose={() => setSuccessMessage(null)}
                message={successMessage}
            />

            <Box sx={{ display: 'flex', gap: 3, flexWrap: 'wrap' }}>
                {/* 左侧文件列表 */}
                <Paper sx={{ width: 300, p: 2 }}>
                    <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
                        <Typography variant="h6">文件列表</Typography>
                        <IconButton onClick={loadFiles} disabled={loading} size="small">
                            <RefreshIcon />
                        </IconButton>
                    </Box>

                    {/* 上传区域 */}
                    <Box sx={{ mb: 2 }}>
                        <Button
                            variant="outlined"
                            component="label"
                            startIcon={<UploadIcon />}
                            fullWidth
                            disabled={uploading}
                        >
                            {uploading ? '上传中...' : 'Upload .txt'}
                            <Input type="file" onChange={handleUpload} sx={{ display: 'none' }} />
                        </Button>
                    </Box>

                    <Divider sx={{ mb: 2 }} />

                    {/* 文件列表 */}
                    {loading && !files.length ? (
                        <CircularProgress size={24} />
                    ) : files.length === 0 ? (
                        <Typography color="text.secondary">未找到TXT文件</Typography>
                    ) : (
                        <List dense>
                            {files.map((file) => (
                                <ListItem
                                    key={file}
                                    button
                                    selected={selectedFile === file}
                                    onClick={() => handleReadFile(file)}
                                >
                                    <ListItemText primary={file} />
                                    <ListItemSecondaryAction>
                                        <IconButton
                                            edge="end"
                                            aria-label="delete"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                handleDeleteFile(file);
                                            }}
                                            size="small"
                                        >
                                            <DeleteIcon fontSize="small" />
                                        </IconButton>
                                    </ListItemSecondaryAction>
                                </ListItem>
                            ))}
                        </List>
                    )}
                </Paper>

                {/* 右侧编辑器 */}
                <Paper sx={{ flex: 1, p: 2 }}>
                    <Typography variant="h6" gutterBottom>
                        {selectedFile ? `编辑中: ${selectedFile}` : '创建新文件'}
                    </Typography>

                    {!selectedFile && (
                        <TextField
                            fullWidth
                            label="New filename (include .txt)"
                            value={newFileName}
                            onChange={(e) => setNewFileName(e.target.value)}
                            margin="normal"
                            size="small"
                            helperText="Enter filename ending with .txt"
                        />
                    )}

                    <TextField
                        fullWidth
                        multiline
                        rows={20}
                        variant="outlined"
                        value={fileContent}
                        onChange={(e) => setFileContent(e.target.value)}
                        placeholder="File content..."
                        sx={{ fontFamily: 'monospace', mt: 2 }}
                    />

                    <Box sx={{ display: 'flex', gap: 2, justifyContent: 'flex-end', mt: 2 }}>
                        <Button
                            variant="contained"
                            startIcon={<SaveIcon />}
                            onClick={handleSaveFile}
                            disabled={loading || (!selectedFile && !newFileName)}
                        >
                            Save
                        </Button>
                        {(selectedFile || newFileName) && (
                            <Button
                                variant="outlined"
                                startIcon={<CancelIcon />}
                                onClick={handleCancel}
                            >
                                Cancel
                            </Button>
                        )}
                    </Box>
                </Paper>
            </Box>
        </Box>
    );
};

export default FileManager;