#include "pch.h"
#include "CubismRenderer_DX12.h"
#include "Live2DModelDX12.h"
#include <d3dcompiler.h>
#include <d3dx12.h>
#include <fstream>
#include "cubism_model_setting_json.hpp"

using namespace Csm;

namespace Live2D_Native
{
    // 顶点着色器HLSL
    const char* vertexShaderSource = R"(
        cbuffer ConstantBuffer : register(b0)
        {
            float4x4 modelMatrix;
            float4x4 clipMatrix;
            float4 baseColor;
            float4 screenColor;
            float4 multiplyColor;
            float4 channelFlag;
        };
        
        struct VSInput
        {
            float2 position : POSITION;
            float2 uv : TEXCOORD0;
        };
        
        struct PSInput
        {
            float4 position : SV_POSITION;
            float2 uv : TEXCOORD0;
        };
        
        PSInput VSMain(VSInput input)
        {
            PSInput output;
            
            // 应用模型矩阵
            float4 position = float4(input.position, 0.0, 1.0);
            output.position = mul(modelMatrix, position);
            
            // 传递UV坐标
            output.uv = input.uv;
            
            return output;
        }
    )";

    // 像素着色器HLSL（标准绘制）
    const char* pixelShaderSource = R"(
        cbuffer ConstantBuffer : register(b0)
        {
            float4x4 modelMatrix;
            float4x4 clipMatrix;
            float4 baseColor;
            float4 screenColor;
            float4 multiplyColor;
            float4 channelFlag;
        };
        
        Texture2D texture0 : register(t0);
        SamplerState sampler0 : register(s0);
        
        struct PSInput
        {
            float4 position : SV_POSITION;
            float2 uv : TEXCOORD0;
        };
        
        float4 PSMain(PSInput input) : SV_TARGET
        {
            // 采样纹理
            float4 texColor = texture0.Sample(sampler0, input.uv);
            
            // 应用基础颜色
            float4 color = texColor * baseColor;
            
            // 应用屏幕颜色
            color.rgb = color.rgb * (1.0 - screenColor.rgb) + screenColor.rgb;
            
            // 应用乘法颜色
            color.rgb = color.rgb * multiplyColor.rgb;
            
            return color;
        }
    )";

    // 像素着色器HLSL（剪裁遮罩）
    const char* maskPixelShaderSource = R"(
        cbuffer ConstantBuffer : register(b0)
        {
            float4x4 modelMatrix;
            float4x4 clipMatrix;
            float4 baseColor;
            float4 screenColor;
            float4 multiplyColor;
            float4 channelFlag;
        };
        
        Texture2D texture0 : register(t0);
        SamplerState sampler0 : register(s0);
        
        struct PSInput
        {
            float4 position : SV_POSITION;
            float2 uv : TEXCOORD0;
        };
        
        float4 PSMain(PSInput input) : SV_TARGET
        {
            // 采样纹理
            float4 texColor = texture0.Sample(sampler0, input.uv);
            
            // 应用基础颜色
            float4 color = texColor * baseColor;
            
            // 应用屏幕颜色
            color.rgb = color.rgb * (1.0 - screenColor.rgb) + screenColor.rgb;
            
            // 应用乘法颜色
            color.rgb = color.rgb * multiplyColor.rgb;
            
            // 应用通道标志
            color.rgb *= channelFlag.rgb;
            
            return color;
        }
    )";

    CubismRenderer_DX12* CubismRenderer_DX12::Create(CubismModel* model, D3D12Renderer* renderer)
    {
        if (!model || !renderer)
        {
            return nullptr;
        }

        CubismRenderer_DX12* rendererDX12 = new CubismRenderer_DX12(model, renderer);
        rendererDX12->Initialize(model);

        return rendererDX12;
    }

    CubismRenderer_DX12::CubismRenderer_DX12(CubismModel* model, D3D12Renderer* renderer)
        : m_renderer(renderer)
        , m_model(model)
        , m_constantBufferData(nullptr)
    {
    }

    CubismRenderer_DX12::~CubismRenderer_DX12()
    {
    }

    void CubismRenderer_DX12::Initialize(CubismModel* model)
    {
        // 初始化着色器
        InitializeShader();

        // 创建根签名
        CreateRootSignature();

        // 创建管线状态对象
        CreatePipelineStateObjects();

        // 创建常量缓冲区
        CreateConstantBuffer();

        // 初始化混合状态
        m_blendDesc.AlphaToCoverageEnable = FALSE;
        m_blendDesc.IndependentBlendEnable = FALSE;

        // 设置混合模式
        for (UINT i = 0; i < D3D12_SIMULTANEOUS_RENDER_TARGET_COUNT; ++i)
        {
            m_blendDesc.RenderTarget[i].BlendEnable = TRUE;
            m_blendDesc.RenderTarget[i].LogicOpEnable = FALSE;
            m_blendDesc.RenderTarget[i].SrcBlend = D3D12_BLEND_SRC_ALPHA;
            m_blendDesc.RenderTarget[i].DestBlend = D3D12_BLEND_INV_SRC_ALPHA;
            m_blendDesc.RenderTarget[i].BlendOp = D3D12_BLEND_OP_ADD;
            m_blendDesc.RenderTarget[i].SrcBlendAlpha = D3D12_BLEND_ONE;
            m_blendDesc.RenderTarget[i].DestBlendAlpha = D3D12_BLEND_ZERO;
            m_blendDesc.RenderTarget[i].BlendOpAlpha = D3D12_BLEND_OP_ADD;
            m_blendDesc.RenderTarget[i].LogicOp = D3D12_LOGIC_OP_NOOP;
            m_blendDesc.RenderTarget[i].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL;
        }
    }

    void CubismRenderer_DX12::InitializeShader()
    {
        // 编译顶点着色器
        HRESULT hr = D3DCompile(
            vertexShaderSource,
            strlen(vertexShaderSource),
            nullptr,
            nullptr,
            nullptr,
            "VSMain",
            "vs_5_0",
            0,
            0,
            &m_vertexShader,
            nullptr
        );

        if (FAILED(hr))
        {
            return;
        }

        // 编译像素着色器（标准绘制）
        hr = D3DCompile(
            pixelShaderSource,
            strlen(pixelShaderSource),
            nullptr,
            nullptr,
            nullptr,
            "PSMain",
            "ps_5_0",
            0,
            0,
            &m_pixelShader,
            nullptr
        );

        if (FAILED(hr))
        {
            return;
        }

        // 编译像素着色器（剪裁遮罩）
        hr = D3DCompile(
            maskPixelShaderSource,
            strlen(maskPixelShaderSource),
            nullptr,
            nullptr,
            nullptr,
            "PSMain",
            "ps_5_0",
            0,
            0,
            &m_maskPixelShader,
            nullptr
        );

        if (FAILED(hr))
        {
            return;
        }
    }

    void CubismRenderer_DX12::CreateRootSignature()
    {
        // 创建根签名描述
        CD3DX12_ROOT_PARAMETER1 rootParameters[2];

        // 常量缓冲区（使用描述符表）
        CD3DX12_DESCRIPTOR_RANGE1 cbvRange;
        cbvRange.Init(D3D12_DESCRIPTOR_RANGE_TYPE_CBV, 1, 0, 0, D3D12_DESCRIPTOR_RANGE_FLAG_DATA_STATIC);
        rootParameters[0].InitAsDescriptorTable(1, &cbvRange, D3D12_SHADER_VISIBILITY_ALL);

        // 着色器资源视图（纹理）
        CD3DX12_DESCRIPTOR_RANGE1 srvRange;
        srvRange.Init(D3D12_DESCRIPTOR_RANGE_TYPE_SRV, 1, 0, 0, D3D12_DESCRIPTOR_RANGE_FLAG_DATA_STATIC);
        rootParameters[1].InitAsDescriptorTable(1, &srvRange, D3D12_SHADER_VISIBILITY_PIXEL);

        // 采样器
        CD3DX12_STATIC_SAMPLER_DESC sampler(
            0, // shaderRegister
            D3D12_FILTER_MIN_MAG_MIP_LINEAR, // filter
            D3D12_TEXTURE_ADDRESS_MODE_CLAMP, // addressU
            D3D12_TEXTURE_ADDRESS_MODE_CLAMP, // addressV
            D3D12_TEXTURE_ADDRESS_MODE_CLAMP, // addressW
            0.0f, // mipLODBias
            16, // maxAnisotropy
            D3D12_COMPARISON_FUNC_NEVER, // comparisonFunc
            D3D12_STATIC_BORDER_COLOR_OPAQUE_BLACK, // borderColor
            0.0f, // minLOD
            D3D12_FLOAT32_MAX // maxLOD
        );

        // 创建根签名描述
        CD3DX12_VERSIONED_ROOT_SIGNATURE_DESC rootSignatureDesc;
        rootSignatureDesc.Init_1_1(
            _countof(rootParameters),
            rootParameters,
            1,
            &sampler,
            D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT |
            D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS |
            D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS |
            D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS
        );

        // 序列化根签名
        ComPtr<ID3DBlob> signature;
        ComPtr<ID3DBlob> error;

        HRESULT hr = D3DX12SerializeVersionedRootSignature(
            &rootSignatureDesc,
            D3D_ROOT_SIGNATURE_VERSION_1_1,
            &signature,
            &error
        );

        if (FAILED(hr))
        {
            return;
        }

        // 创建根签名
        ID3D12Device* device = m_renderer->GetDevice();
        hr = device->CreateRootSignature(
            0,
            signature->GetBufferPointer(),
            signature->GetBufferSize(),
            IID_PPV_ARGS(&m_rootSignature)
        );

        if (FAILED(hr))
        {
            return;
        }
    }

    void CubismRenderer_DX12::CreatePipelineStateObjects()
    {
        ID3D12Device* device = m_renderer->GetDevice();

        // 输入元素描述
        D3D12_INPUT_ELEMENT_DESC inputElementDescs[] =
        {
            { "POSITION", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 0, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 },
            { "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 8, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, 0 }
        };

        // 创建管线状态对象（标准绘制）
        D3D12_GRAPHICS_PIPELINE_STATE_DESC psoDesc = {};
        psoDesc.InputLayout = { inputElementDescs, _countof(inputElementDescs) };
        psoDesc.pRootSignature = m_rootSignature.Get();
        psoDesc.VS = CD3DX12_SHADER_BYTECODE(m_vertexShader.Get());
        psoDesc.PS = CD3DX12_SHADER_BYTECODE(m_pixelShader.Get());
        psoDesc.RasterizerState = CD3DX12_RASTERIZER_DESC(D3D12_DEFAULT);
        psoDesc.BlendState = m_blendDesc;
        psoDesc.DepthStencilState.DepthEnable = FALSE;
        psoDesc.DepthStencilState.StencilEnable = FALSE;
        psoDesc.SampleMask = UINT_MAX;
        psoDesc.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
        psoDesc.NumRenderTargets = 1;
        psoDesc.RTVFormats[0] = m_renderer->GetBackBufferFormat();
        psoDesc.SampleDesc.Count = 1;

        HRESULT hr = device->CreateGraphicsPipelineState(&psoDesc, IID_PPV_ARGS(&m_pipelineState));
        if (FAILED(hr))
        {
            return;
        }

        // 创建管线状态对象（剪裁遮罩）
        psoDesc.PS = CD3DX12_SHADER_BYTECODE(m_maskPixelShader.Get());
        hr = device->CreateGraphicsPipelineState(&psoDesc, IID_PPV_ARGS(&m_maskPipelineState));
        if (FAILED(hr))
        {
            return;
        }
    }

    void CubismRenderer_DX12::CreateConstantBuffer()
    {
        ID3D12Device* device = m_renderer->GetDevice();

        // 创建常量缓冲区
        D3D12_HEAP_PROPERTIES heapProps = {};
        heapProps.Type = D3D12_HEAP_TYPE_UPLOAD;
        heapProps.CreationNodeMask = 1;
        heapProps.VisibleNodeMask = 1;

        D3D12_RESOURCE_DESC constantBufferDesc = {};
        constantBufferDesc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
        constantBufferDesc.Alignment = 0;
        constantBufferDesc.Width = sizeof(ConstantBuffer);
        constantBufferDesc.Height = 1;
        constantBufferDesc.DepthOrArraySize = 1;
        constantBufferDesc.MipLevels = 1;
        constantBufferDesc.Format = DXGI_FORMAT_UNKNOWN;
        constantBufferDesc.SampleDesc.Count = 1;
        constantBufferDesc.SampleDesc.Quality = 0;
        constantBufferDesc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
        constantBufferDesc.Flags = D3D12_RESOURCE_FLAG_NONE;

        HRESULT hr = device->CreateCommittedResource(
            &heapProps,
            D3D12_HEAP_FLAG_NONE,
            &constantBufferDesc,
            D3D12_RESOURCE_STATE_GENERIC_READ,
            nullptr,
            IID_PPV_ARGS(&m_constantBuffer)
        );

        if (FAILED(hr))
        {
            return;
        }

        // 映射常量缓冲区
        D3D12_RANGE readRange = { 0, 0 };
        hr = m_constantBuffer->Map(0, &readRange, reinterpret_cast<void**>(&m_constantBufferData));
        if (FAILED(hr))
        {
            return;
        }

        // 创建单独的 CBV 描述符堆
        D3D12_DESCRIPTOR_HEAP_DESC cbvHeapDesc = {};
        cbvHeapDesc.NumDescriptors = 1;
        cbvHeapDesc.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
        cbvHeapDesc.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;

        hr = device->CreateDescriptorHeap(&cbvHeapDesc, IID_PPV_ARGS(&m_cbvHeap));
        if (FAILED(hr))
        {
            return;
        }

        // 创建常量缓冲区视图
        D3D12_CPU_DESCRIPTOR_HANDLE cpuHandle = m_cbvHeap->GetCPUDescriptorHandleForHeapStart();

        D3D12_CONSTANT_BUFFER_VIEW_DESC cbvDesc = {};
        cbvDesc.BufferLocation = m_constantBuffer->GetGPUVirtualAddress();
        cbvDesc.SizeInBytes = (sizeof(ConstantBuffer) + 255) & ~255; // 必须是256字节的倍数

        device->CreateConstantBufferView(&cbvDesc, cpuHandle);

        // 保存GPU句柄
        m_constantBufferView = m_cbvHeap->GetGPUDescriptorHandleForHeapStart();
    }

    void CubismRenderer_DX12::UpdateConstantBuffer(int drawableIndex)
    {
        if (!m_model || !m_constantBufferData)
        {
            return;
        }

        // 获取模型矩阵
        const csmFloat32* modelMatrix = m_model->GetModelMatrix()->GetArray();

        // 获取剪裁矩阵
        const csmFloat32* clipMatrix = m_model->GetClipMatrix()->GetArray();

        // 获取基础颜色
        csmFloat32 opacity = m_model->GetDrawableOpacity(drawableIndex);
        const csmFloat32* baseColor = m_model->GetDrawableMultiplyColor(drawableIndex);

        // 获取屏幕颜色
        const csmFloat32* screenColor = m_model->GetDrawableScreenColor(drawableIndex);

        // 获取乘法颜色
        const csmFloat32* multiplyColor = m_model->GetDrawableMultiplyColor(drawableIndex);

        // 更新常量缓冲区
        memcpy(m_constantBufferData->modelMatrix, modelMatrix, sizeof(float) * 16);
        memcpy(m_constantBufferData->clipMatrix, clipMatrix, sizeof(float) * 16);

        m_constantBufferData->baseColor[0] = baseColor[0];
        m_constantBufferData->baseColor[1] = baseColor[1];
        m_constantBufferData->baseColor[2] = baseColor[2];
        m_constantBufferData->baseColor[3] = baseColor[3] * opacity;

        m_constantBufferData->screenColor[0] = screenColor[0];
        m_constantBufferData->screenColor[1] = screenColor[1];
        m_constantBufferData->screenColor[2] = screenColor[2];
        m_constantBufferData->screenColor[3] = screenColor[3];

        m_constantBufferData->multiplyColor[0] = multiplyColor[0];
        m_constantBufferData->multiplyColor[1] = multiplyColor[1];
        m_constantBufferData->multiplyColor[2] = multiplyColor[2];
        m_constantBufferData->multiplyColor[3] = multiplyColor[3];

        // 获取通道标志
        const csmInt32* drawableMasks = m_model->GetDrawableDrawableMasks(drawableIndex);
        int drawableMaskCount = m_model->GetDrawableDrawableMaskCounts(drawableIndex);

        m_constantBufferData->channelFlag[0] = 0.0f;
        m_constantBufferData->channelFlag[1] = 0.0f;
        m_constantBufferData->channelFlag[2] = 0.0f;
        m_constantBufferData->channelFlag[3] = 0.0f;

        for (int i = 0; i < drawableMaskCount; i++)
        {
            int maskIndex = drawableMasks[i];
            int channel = m_model->GetDrawableMaskInvertedMask(maskIndex) ? 1 : 0;
            int colorChannel = m_model->GetDrawableMaskChannel(maskIndex);

            m_constantBufferData->channelFlag[colorChannel] = static_cast<float>(channel);
        }
    }

    void CubismRenderer_DX12::PreDraw()
    {
        ID3D12GraphicsCommandList* commandList = m_renderer->GetCommandList();

        // 设置根签名
        commandList->SetGraphicsRootSignature(m_rootSignature.Get());

        // 设置管线状态对象
        commandList->SetPipelineState(m_pipelineState.Get());

        // 设置图元拓扑
        commandList->IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

        // 设置常量缓冲区描述符表
        commandList->SetGraphicsRootDescriptorTable(0, m_constantBufferView);
    }

    void CubismRenderer_DX12::PostDraw()
    {
        // 清除状态
        ID3D12GraphicsCommandList* commandList = m_renderer->GetCommandList();
        commandList->SetPipelineState(nullptr);
    }

    void CubismRenderer_DX12::DrawModel()
    {
        if (!m_model)
        {
            return;
        }

        // 打开命令列表
        if (FAILED(m_renderer->OpenCommandList()))
        {
            return;
        }

        // 准备绘制
        PreDraw();

        // 设置渲染目标
        ID3D12GraphicsCommandList* commandList = m_renderer->GetCommandList();
        UINT frameIndex = m_renderer->GetFrameIndex();
        ID3D12Resource* renderTarget = m_renderer->GetRenderTarget(frameIndex);

        D3D12_RESOURCE_BARRIER barrier = {};
        barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
        barrier.Flags = D3D12_RESOURCE_BARRIER_FLAG_NONE;
        barrier.Transition.pResource = renderTarget;
        barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_PRESENT;
        barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_RENDER_TARGET;
        barrier.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;

        commandList->ResourceBarrier(1, &barrier);

        // 设置渲染目标视图
        D3D12_CPU_DESCRIPTOR_HANDLE rtvHandle = m_renderer->GetRTVHeap()->GetCPUDescriptorHandleForHeapStart();
        rtvHandle.ptr += frameIndex * m_renderer->GetRTVDescriptorSize();

        commandList->OMSetRenderTargets(1, &rtvHandle, FALSE, nullptr);

        // 清除渲染目标
        const float clearColor[] = { 0.0f, 0.0f, 0.0f, 0.0f };
        commandList->ClearRenderTargetView(rtvHandle, clearColor, 0, nullptr);

        // 设置视口和剪裁矩形
        D3D12_VIEWPORT viewport = {};
        viewport.TopLeftX = 0;
        viewport.TopLeftY = 0;
        viewport.Width = static_cast<float>(m_renderer->GetWidth());
        viewport.Height = static_cast<float>(m_renderer->GetHeight());
        viewport.MinDepth = D3D12_MIN_DEPTH;
        viewport.MaxDepth = D3D12_MAX_DEPTH;

        commandList->RSSetViewports(1, &viewport);

        D3D12_RECT scissorRect = {};
        scissorRect.left = 0;
        scissorRect.top = 0;
        scissorRect.right = m_renderer->GetWidth();
        scissorRect.bottom = m_renderer->GetHeight();

        commandList->RSSetScissorRects(1, &scissorRect);

        // 绘制所有可绘制对象
        int drawableCount = m_model->GetDrawableCount();
        for (int i = 0; i < drawableCount; i++)
        {
            DrawMesh(i);
        }

        // 结束绘制
        PostDraw();

        // 转换资源状态
        barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_RENDER_TARGET;
        barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_PRESENT;
        commandList->ResourceBarrier(1, &barrier);

        // 关闭命令列表
        m_renderer->CloseCommandList();
    }

    void CubismRenderer_DX12::DrawMesh(int drawableIndex)
    	{
    		if (!m_model)
    		{
    			return;
    		}
    
    		// 检查可绘制对象是否可见
    		if (!m_model->GetDrawableDynamicFlagIsVisible(drawableIndex))
    		{
    			return;
    		}
    
    		// 更新常量缓冲区
    		UpdateConstantBuffer(drawableIndex);
    
    		// 设置顶点和索引缓冲区
    		ID3D12GraphicsCommandList* commandList = m_renderer->GetCommandList();
    
    		// 获取顶点和索引缓冲区
    		ComPtr<ID3D12Resource> vertexBuffer = m_renderer->GetVertexBuffer(drawableIndex);
    		ComPtr<ID3D12Resource> indexBuffer = m_renderer->GetIndexBuffer(drawableIndex);
    
    		if (!vertexBuffer || !indexBuffer)
    		{
    			return;
    		}
    
    		// 设置顶点缓冲区
    		D3D12_VERTEX_BUFFER_VIEW vbv = {};
    		vbv.BufferLocation = vertexBuffer->GetGPUVirtualAddress();
    		vbv.StrideInBytes = sizeof(CubismVertex);
    		vbv.SizeInBytes = m_model->GetDrawableVertexCount(drawableIndex) * sizeof(CubismVertex);
    
    		commandList->IASetVertexBuffers(0, 1, &vbv);
    
    		// 设置索引缓冲区
    		D3D12_INDEX_BUFFER_VIEW ibv = {};
    		ibv.BufferLocation = indexBuffer->GetGPUVirtualAddress();
    		ibv.Format = DXGI_FORMAT_R16_UINT;
    		ibv.SizeInBytes = m_model->GetDrawableVertexIndexCount(drawableIndex) * sizeof(csmUint16);
    
    		commandList->IASetIndexBuffer(&ibv);
    
    		// 设置纹理
    		int textureIndex = m_model->GetDrawableTextureIndex(drawableIndex);
    		D3D12_GPU_DESCRIPTOR_HANDLE textureSrv = m_renderer->GetTextureSrvHandle(textureIndex);
    		commandList->SetGraphicsRootDescriptorTable(1, textureSrv);
    
    		// 获取混合模式
    		CubismBlendMode blendMode = m_model->GetDrawableBlendMode(drawableIndex);
    		bool isPremultiplied = m_model->GetDrawableInvertedMaskBit(drawableIndex);
    
    		// 使用缓存的管线状态对象
    		ID3D12PipelineState* pipelineState = GetOrCreatePipelineState(blendMode, isPremultiplied);
    		if (pipelineState)
    		{
    			commandList->SetPipelineState(pipelineState);
    		}
    
    		// 绘制
    		commandList->DrawIndexedInstanced(
    			m_model->GetDrawableVertexIndexCount(drawableIndex),
    			1,
    			0,
    			0,
    			0
    		);
    	}
    
    	ID3D12PipelineState* CubismRenderer_DX12::GetOrCreatePipelineState(CubismBlendMode blendMode, bool isPremultiplied)
    	{
    		// 计算缓存键
    		int cacheKey = (static_cast<int>(blendMode) << 1) | (isPremultiplied ? 1 : 0);
    
    		// 检查缓存
    		if (cacheKey < static_cast<int>(m_cachedPipelineStates.size()) && m_cachedPipelineStates[cacheKey])
    		{
    			return m_cachedPipelineStates[cacheKey].Get();
    		}
    
    		// 确保缓存足够大
    		if (cacheKey >= static_cast<int>(m_cachedPipelineStates.size()))
    		{
    			m_cachedPipelineStates.resize(cacheKey + 1);
    		}
    
    		// 设置混合模式
    		if (blendMode == CubismBlendMode_Additive)
    		{
    			// 加法混合
    			m_blendDesc.RenderTarget[0].SrcBlend = D3D12_BLEND_SRC_ALPHA;
    			m_blendDesc.RenderTarget[0].DestBlend = D3D12_BLEND_ONE;
    			m_blendDesc.RenderTarget[0].BlendOp = D3D12_BLEND_OP_ADD;
    		}
    		else if (blendMode == CubismBlendMode_Multiplicative)
    		{
    			// 乘法混合
    			m_blendDesc.RenderTarget[0].SrcBlend = D3D12_BLEND_DEST_COLOR;
    			m_blendDesc.RenderTarget[0].DestBlend = D3D12_BLEND_SRC_COLOR;
    			m_blendDesc.RenderTarget[0].BlendOp = D3D12_BLEND_OP_ADD;
    		}
    		else
    		{
    			// 正常混合
    			m_blendDesc.RenderTarget[0].SrcBlend = D3D12_BLEND_SRC_ALPHA;
    			m_blendDesc.RenderTarget[0].DestBlend = D3D12_BLEND_INV_SRC_ALPHA;
    			m_blendDesc.RenderTarget[0].BlendOp = D3D12_BLEND_OP_ADD;
    		}
    
    		// 创建管线状态对象
    		D3D12_GRAPHICS_PIPELINE_STATE_DESC psoDesc = {};
    		psoDesc.InputLayout = { nullptr, 0 };
    		psoDesc.pRootSignature = m_rootSignature.Get();
    		psoDesc.VS = CD3DX12_SHADER_BYTECODE(m_vertexShader.Get());
    		psoDesc.PS = CD3DX12_SHADER_BYTECODE(m_pixelShader.Get());
    		psoDesc.RasterizerState = CD3DX12_RASTERIZER_DESC(D3D12_DEFAULT);
    		psoDesc.BlendState = m_blendDesc;
    		psoDesc.DepthStencilState.DepthEnable = FALSE;
    		psoDesc.DepthStencilState.StencilEnable = FALSE;
    		psoDesc.SampleMask = UINT_MAX;
    		psoDesc.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
    		psoDesc.NumRenderTargets = 1;
    		psoDesc.RTVFormats[0] = m_renderer->GetBackBufferFormat();
    		psoDesc.SampleDesc.Count = 1;
    
    		HRESULT hr = m_renderer->GetDevice()->CreateGraphicsPipelineState(&psoDesc, IID_PPV_ARGS(&m_cachedPipelineStates[cacheKey]));
    		if (FAILED(hr))
    		{
    			return nullptr;
    		}
    
    		return m_cachedPipelineStates[cacheKey].Get();
    	}}
