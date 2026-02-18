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

export interface FileResponse {
    file_name: string;
    file_path: string;
    file_size: number;
    file_type: string;
    file_content: string;
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

    // 获取文件列表
    async listFiles(): Promise<FileResponse> {
        const response = await api.get<FileResponse>('/api/files');
        return response.data;
    },

    // 查看文件内容
    async readFile(fileName: string): Promise<FileResponse> {
        const response = await api.get<FileResponse>(`/api/files/${fileName}`);
        return response.data;
    },

    // 创建或更新文件
    async createOrUpdateFile(fileName: string, fileContent: string): Promise<FileResponse> {
        const response = await api.put<FileResponse>(`/api/files/${fileName}`, {
            file_name: fileName,
            file_content: fileContent,
        });
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
};