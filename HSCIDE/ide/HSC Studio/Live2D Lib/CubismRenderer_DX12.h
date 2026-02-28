#pragma once
#include "D3D12Renderer.h"
#include "Live2DCubismCore.h"
#include <d3d12.h>
#include <wrl/client.h>
#include <vector>

using Microsoft::WRL::ComPtr;

namespace Live2D_Native
{
    // 顶点结构
    struct CubismVertex
    {
        float position[2];
        float uv[2];
    };

    // 常量缓冲区结构
    struct ConstantBuffer
    {
        float modelMatrix[16];
        float clipMatrix[16];
        float baseColor[4];
        float screenColor[4];
        float multiplyColor[4];
        float channelFlag[4];
    };

    // 混合模式枚举
    enum CubismBlendMode
    {
        CubismBlendMode_Normal,
        CubismBlendMode_Additive,
        CubismBlendMode_Multiplicative
    };

    class CubismRenderer_DX12
    {
    public:
        static CubismRenderer_DX12* Create(csmModel* model, D3D12Renderer* renderer);

        void Initialize(csmModel* model);
        void DrawModel();

        void PreDraw();
        void PostDraw();
        void DrawMesh(int drawableIndex);

    protected:
        CubismRenderer_DX12(csmModel* model, D3D12Renderer* renderer);
        ~CubismRenderer_DX12();

    private:
        void InitializeShader();
        void CreatePipelineStateObjects();
        void CreateRootSignature();
        void CreateConstantBuffer();
        void UpdateConstantBuffer(int drawableIndex);

    private:
        D3D12Renderer* m_renderer; // 不拥有所有权
        csmModel* m_model; // 不拥有所有权

        // 着色器资源
        ComPtr<ID3DBlob> m_vertexShader;
        ComPtr<ID3DBlob> m_pixelShader;
        ComPtr<ID3DBlob> m_maskPixelShader;

        // 管线状态对象
        		ComPtr<ID3D12PipelineState> m_pipelineState;
        		ComPtr<ID3D12PipelineState> m_maskPipelineState;
        		std::vector<ComPtr<ID3D12PipelineState>> m_cachedPipelineStates;
        
        		// 根签名
        		ComPtr<ID3D12RootSignature> m_rootSignature;
        
        		// 常量缓冲区
        		ComPtr<ID3D12Resource> m_constantBuffer;
        		ConstantBuffer* m_constantBufferData;
        
        		// 着色器常量缓冲区视图
        		D3D12_GPU_DESCRIPTOR_HANDLE m_constantBufferView;

        		// CBV 描述符堆
        		ComPtr<ID3D12DescriptorHeap> m_cbvHeap;

        		// 混合状态
        		D3D12_BLEND_DESC m_blendDesc;
        
        		// 获取或创建管线状态对象
        ID3D12PipelineState* GetOrCreatePipelineState(CubismBlendMode blendMode, bool isPremultiplied);
    };
}