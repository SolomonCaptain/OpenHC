import axios from 'axios';

const API_BASE_URL = 'http://154.37.219.104:8000';

// 创建axios实例
const api = axios.create({
    baseURL: API_BASE_URL,
    headers: {
        'Content-Type': 'application/json',
    },
});

// API相应类型定义
export interface HelloResponse {
    message: string;
    source: string;
}

export interface HealthResponse {
    status: string;
    cpp_library_available: boolean;
    service: string;
}

export interface FileInfo {
    file_name: string;
    file_path: string;
    file_size: number;
    file_type: string;
    file_content?: string;
}

export interface FileContentResponse {
    filename: string;
    content: string;
}

export interface UploadResponse {
    message: string;
    filename: string;
}

// API调用函数
export const apiService = {
    // 获取Hello World消息
    async getHello(): Promise<HelloResponse> {
        const response = await api.get<HelloResponse>('/api/hello');
        return response.data;
    },
    
    // 健康检查
    async checkHealth(): Promise<HealthResponse> {
        const response = await api.get<HealthResponse>('/api/health');
        return response.data;
    },

    // 检查服务状态
    async checkServiceStatus(): Promise<{
        isRunning: boolean;
        cppAvailable: boolean;
    }> {
        try {
            const health = await this.checkHealth();
            return {
                isRunning: health.status === 'healthy',
                cppAvailable: health.cpp_library_available,
            };
        } catch (error) {
            console.error('Service check failed:', error);
            return {
                isRunning: false,
                cppAvailable: false,
            };
         }
    },

    // 获取文件列表
    async listFiles(): Promise<string[]> {
        const response = await api.get<string[]>('/api/files');
        return response.data;
    },

    // 读取文件内容
    async readFile(filename: string): Promise<FileContentResponse> {
        const response = await api.get<FileContentResponse>(`/api/files/${encodeURIComponent(filename)}`);
        return response.data;
    },

    // 创建或更新文件
    async saveFile(filename: string, content: string): Promise<{ message: string }> {
        const response = await api.post(`/api/files/${encodeURIComponent(filename)}`, { content });
        return response.data;
    },

    // 删除文件
    async deleteFile(filename: string): Promise<{ message: string }> {
        const response = await api.delete(`/api/files/${encodeURIComponent(filename)}`);
        return response.data;
    },

    // 上传文件
    async uploadFile(file: File): Promise<UploadResponse> {
        const formData = new FormData();
        formData.append('file', file);
        const response = await api.post<UploadResponse>('/api/upload', formData, {
            headers: {
                'Content-Type': 'multipart/form-data',
            },
        });
        return response.data;
    },
};