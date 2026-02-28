#pragma once
#include "D3D12Renderer.h"
#include <Live2DCubismCore.h>
#include <vector>
#include <string>
#include <memory>

namespace Live2D_Native
{
	class Live2DModelDX12
	{
	public:
		Live2DModelDX12(D3D12Renderer* renderer);
		~Live2DModelDX12();

		// 加载模型
		bool LoadModel(const char* moc3Path, const char* physics3Path, const char* model3Json);

		// 更新模型
		void Update(float deltaTime);

		// 渲染模型
		void Render();

		// 设置参数
		void SetParameter(const char* id, float value);

	private:
		// 初始化模型资源
		void InitializeModelResources();

		// 加载纹理
		bool LoadTexture(const char* texturePath, int textureIndex);

		// 创建顶点和索引缓冲区
		void CreateVertexIndexBuffers();

		// 更新顶点缓冲区
		void UpdateVertexBuffer();

		// 渲染单个可绘制对象
		void DrawDrawable(int drawableIndex);

		// 渲染剪裁遮罩
		void RenderClipMasks();

	private:
		D3D12Renderer* m_renderer; // 不拥有所有权
		Csm::CubismModel* m_model;
		Csm::CubismRenderer* m_cubismRenderer;

		// 纹理资源
		std::vector<ComPtr<ID3D12Resource>> m_textures;

		// 顶点和索引缓冲区
		std::vector<ComPtr<ID3D12Resource>> m_vertexBuffers;
		std::vector<ComPtr<ID3D12Resource>> m_indexBuffers;

		// 描述符句柄
		D3D12_GPU_DESCRIPTOR_HANDLE m_textureSrvHandles[64]; // 假设最多支持64个纹理

		// 模型信息
		int m_drawableCount;
		int m_vertexCount;
		int m_indexCount;

		// 模型文件路径
		std::string m_modelDir;
	};
}