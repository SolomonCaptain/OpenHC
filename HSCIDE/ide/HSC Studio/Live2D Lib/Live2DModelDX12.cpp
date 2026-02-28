#include "pch.h"
#include "Live2DModelDX12.h"
#include "CubismRenderer_DX12.h"
#include "Live2DCubismCore.h"
#include "cubism_model_setting_json.hpp"
#include <fstream>
#include <sstream>
#include <d3d12.h>
#include <wincodec.h>

using namespace Csm;

namespace Live2D_Native
{
    Live2DModelDX12::Live2DModelDX12(D3D12Renderer* renderer)
        : m_renderer(renderer)
        , m_model(nullptr)
        , m_cubismRenderer(nullptr)
        , m_drawableCount(0)
        , m_vertexCount(0)
        , m_indexCount(0)
    {
    }

    Live2DModelDX12::~Live2DModelDX12()
    	{
    		// 等待 GPU 完成所有工作
    		if (m_renderer)
    		{
    			m_renderer->WaitForGPU();
    		}
    
    		if (m_cubismRenderer)
    		{
    			delete m_cubismRenderer;
    			m_cubismRenderer = nullptr;
    		}
    
    		if (m_model)
    		{
    			delete m_model;
    			m_model = nullptr;
    		}
    
    		// 清理纹理资源
    		m_textures.clear();
    		m_vertexBuffers.clear();
    		m_indexBuffers.clear();
    	}
    bool Live2DModelDX12::LoadModel(const char* moc3Path, const char* physics3Path, const char* model3Json)
    	{
    		// 参数验证
    		if (!model3Json || !m_renderer)
    		{
    			return false;
    		}
    
    		// 保存模型目录
    		m_modelDir = model3Json;
    		size_t lastSlash = m_modelDir.find_last_of("/\\");
    		if (lastSlash != std::string::npos)
    		{
    			m_modelDir = m_modelDir.substr(0, lastSlash + 1);
    		}
    
    		// 加载模型设置JSON
    		std::ifstream jsonFile(model3Json);
    		if (!jsonFile.is_open())
    		{
    			return false;
    		}
    
    		std::stringstream buffer;
    		buffer << jsonFile.rdbuf();
    		std::string jsonStr = buffer.str();
    		jsonFile.close();
    
    		// 解析JSON
    		CubismModelSettingJson* setting = new CubismModelSettingJson(jsonStr.c_str());
    		if (!setting)
    		{
    			return false;
    		}
    
    		// 加载MOC3文件
    		const char* moc3FileName = setting->GetModelFileName();
    		if (!moc3FileName)
    		{
    			delete setting;
    			return false;
    		}
    
    		std::string moc3File = m_modelDir + moc3FileName;
    		std::ifstream moc3Stream(moc3File, std::ios::binary);
    		if (!moc3Stream.is_open())
    		{
    			delete setting;
    			return false;
    		}
    
    		// 读取文件内容
    		moc3Stream.seekg(0, std::ios::end);
    		size_t moc3Size = moc3Stream.tellg();
    		moc3Stream.seekg(0, std::ios::beg);
    
    		if (moc3Size == 0)
    		{
    			delete setting;
    			return false;
    		}
    
    		csmByte* moc3Buffer = new csmByte[moc3Size];
    		moc3Stream.read(reinterpret_cast<char*>(moc3Buffer), moc3Size);
    		moc3Stream.close();
    
    		// 创建模型
    		m_model = CubismModel::Create(moc3Buffer, moc3Size);
    		delete[] moc3Buffer;
    
    		if (!m_model)
    		{
    			delete setting;
    			return false;
    		}
    
    		// 加载物理设置
    		const char* physicsFileName = setting->GetPhysicsFileName();
    		if (physicsFileName)
    		{
    			std::string physicsFile = m_modelDir + physicsFileName;
    			std::ifstream physicsStream(physicsFile, std::ios::binary);
    			if (physicsStream.is_open())
    			{
    				physicsStream.seekg(0, std::ios::end);
    				size_t physicsSize = physicsStream.tellg();
    				physicsStream.seekg(0, std::ios::beg);
    
    				if (physicsSize > 0)
    				{
    					csmByte* physicsBuffer = new csmByte[physicsSize];
    					physicsStream.read(reinterpret_cast<char*>(physicsBuffer), physicsSize);
    					physicsStream.close();
    
    					// 加载物理设置到模型
    					m_model->LoadPhysics(physicsBuffer, physicsSize);
    					delete[] physicsBuffer;
    				}
    			}
    		}
    
    		// 加载纹理
    		int textureCount = setting->GetTextureCount();
    		if (textureCount <= 0)
    		{
    			delete setting;
    			delete m_model;
    			m_model = nullptr;
    			return false;
    		}
    
    		m_textures.resize(textureCount);
    
    		for (int i = 0; i < textureCount; i++)
    		{
    			const char* textureFileName = setting->GetTextureFileName(i);
    			if (!textureFileName)
    			{
    				delete setting;
    				delete m_model;
    				m_model = nullptr;
    				return false;
    			}
    
    			std::string texturePath = m_modelDir + textureFileName;
    			if (!LoadTexture(texturePath.c_str(), i))
    			{
    				delete setting;
    				delete m_model;
    				m_model = nullptr;
    				return false;
    			}
    		}
    
    		// 创建渲染器
    		m_cubismRenderer = CubismRenderer_DX12::Create(m_model, m_renderer);
    		if (!m_cubismRenderer)
    		{
    			delete setting;
    			delete m_model;
    			m_model = nullptr;
    			return false;
    		}
    
    		// 初始化渲染器
    		m_cubismRenderer->Initialize(m_model);
    
    		// 初始化模型资源
    		InitializeModelResources();
    
    		delete setting;
    		return true;
    	}
    void Live2DModelDX12::InitializeModelResources()
    {
        if (!m_model || !m_cubismRenderer)
        {
            return;
        }

        // 获取可绘制对象数量
        m_drawableCount = m_model->GetDrawableCount();

        // 计算顶点和索引总数
        m_vertexCount = 0;
        m_indexCount = 0;

        for (int i = 0; i < m_drawableCount; i++)
        {
            m_vertexCount += m_model->GetDrawableVertexCount(i);
            m_indexCount += m_model->GetDrawableVertexIndexCount(i);
        }

        // 创建顶点和索引缓冲区
        CreateVertexIndexBuffers();
    }

bool Live2DModelDX12::LoadTexture(const char* texturePath, int textureIndex)
	{
		// 使用 Windows Imaging Component (WIC) 加载纹理
		HRESULT hr = S_OK;

		// 创建 WIC 工厂
		ComPtr<IWICImagingFactory> wicFactory;
		hr = CoCreateInstance(
			CLSID_WICImagingFactory,
			nullptr,
			CLSCTX_INPROC_SERVER,
			IID_PPV_ARGS(&wicFactory)
		);

		if (FAILED(hr))
		{
			return false;
		}

		// 将路径转换为宽字符
		wchar_t wPath[MAX_PATH];
		MultiByteToWideChar(CP_UTF8, 0, texturePath, -1, wPath, MAX_PATH);

		// 创建解码器
		ComPtr<IWICBitmapDecoder> decoder;
		hr = wicFactory->CreateDecoderFromFilename(
			wPath,
			nullptr,
			GENERIC_READ,
			WICDecodeMetadataCacheOnLoad,
			&decoder
		);

		if (FAILED(hr))
		{
			return false;
		}

		// 获取第一帧
		ComPtr<IWICBitmapFrameDecode> frame;
		hr = decoder->GetFrame(0, &frame);
		if (FAILED(hr))
		{
			return false;
		}

		// 获取图像信息
		UINT width, height;
		hr = frame->GetSize(&width, &height);
		if (FAILED(hr))
		{
			return false;
		}

		// 获取像素格式
		WICPixelFormatGUID pixelFormat;
		hr = frame->GetPixelFormat(&pixelFormat);
		if (FAILED(hr))
		{
			return false;
		}

		// 转换为 BGRA8 格式
		ComPtr<IWICFormatConverter> converter;
		hr = wicFactory->CreateFormatConverter(&converter);
		if (FAILED(hr))
		{
			return false;
		}

		hr = converter->Initialize(
			frame.Get(),
			GUID_WICPixelFormat32bppBGRA,
			WICBitmapDitherTypeNone,
			nullptr,
			0.0,
			WICBitmapPaletteTypeCustom
		);

		if (FAILED(hr))
		{
			return false;
		}

		// 计算图像数据大小
		UINT stride = width * 4; // BGRA = 4 bytes per pixel
		UINT imageSize = stride * height;

		// 分配图像数据缓冲区
		std::vector<BYTE> imageData(imageSize);

		// 复制像素数据
		hr = converter->CopyPixels(nullptr, stride, imageSize, imageData.data());
		if (FAILED(hr))
		{
			return false;
		}

		// 创建纹理资源
		D3D12_RESOURCE_DESC textureDesc = {};
		textureDesc.MipLevels = 1;
		textureDesc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
		textureDesc.Width = width;
		textureDesc.Height = height;
		textureDesc.Flags = D3D12_RESOURCE_FLAG_NONE;
		textureDesc.DepthOrArraySize = 1;
		textureDesc.SampleDesc.Count = 1;
		textureDesc.SampleDesc.Quality = 0;
		textureDesc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;

		D3D12_HEAP_PROPERTIES heapProps = {};
		heapProps.Type = D3D12_HEAP_TYPE_DEFAULT;
		heapProps.CreationNodeMask = 1;
		heapProps.VisibleNodeMask = 1;

		ID3D12Device* device = m_renderer->GetDevice();

		hr = device->CreateCommittedResource(
			&heapProps,
			D3D12_HEAP_FLAG_NONE,
			&textureDesc,
			D3D12_RESOURCE_STATE_COPY_DEST,
			nullptr,
			IID_PPV_ARGS(&m_textures[textureIndex])
		);

		if (FAILED(hr))
		{
			return false;
		}

		// 创建上传堆
		D3D12_HEAP_PROPERTIES uploadHeapProps = {};
		uploadHeapProps.Type = D3D12_HEAP_TYPE_UPLOAD;
		uploadHeapProps.CreationNodeMask = 1;
		uploadHeapProps.VisibleNodeMask = 1;

		D3D12_RESOURCE_DESC uploadBufferDesc = {};
		uploadBufferDesc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
		uploadBufferDesc.Alignment = 0;
		uploadBufferDesc.Width = imageSize;
		uploadBufferDesc.Height = 1;
		uploadBufferDesc.DepthOrArraySize = 1;
		uploadBufferDesc.MipLevels = 1;
		uploadBufferDesc.Format = DXGI_FORMAT_UNKNOWN;
		uploadBufferDesc.SampleDesc.Count = 1;
		uploadBufferDesc.SampleDesc.Quality = 0;
		uploadBufferDesc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
		uploadBufferDesc.Flags = D3D12_RESOURCE_FLAG_NONE;

		ComPtr<ID3D12Resource> uploadBuffer;
		hr = device->CreateCommittedResource(
			&uploadHeapProps,
			D3D12_HEAP_FLAG_NONE,
			&uploadBufferDesc,
			D3D12_RESOURCE_STATE_GENERIC_READ,
			nullptr,
			IID_PPV_ARGS(&uploadBuffer)
		);

		if (FAILED(hr))
		{
			return false;
		}

		// 复制纹理数据到上传堆
		void* pData = nullptr;
		D3D12_RANGE readRange = { 0, 0 };
		hr = uploadBuffer->Map(0, &readRange, &pData);
		if (FAILED(hr))
		{
			return false;
		}

		memcpy(pData, imageData.data(), imageSize);
		uploadBuffer->Unmap(0, nullptr);

		// 复制纹理数据
		ID3D12GraphicsCommandList* commandList = m_renderer->GetCommandList();

		D3D12_TEXTURE_COPY_LOCATION srcLocation = {};
		srcLocation.pResource = uploadBuffer.Get();
		srcLocation.Type = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
		srcLocation.PlacedFootprint.Footprint.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
		srcLocation.PlacedFootprint.Footprint.Width = width;
		srcLocation.PlacedFootprint.Footprint.Height = height;
		srcLocation.PlacedFootprint.Footprint.Depth = 1;
		srcLocation.PlacedFootprint.Footprint.RowPitch = stride;

		D3D12_TEXTURE_COPY_LOCATION dstLocation = {};
		dstLocation.pResource = m_textures[textureIndex].Get();
		dstLocation.Type = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
		dstLocation.SubresourceIndex = 0;

		commandList->CopyTextureRegion(&dstLocation, 0, 0, 0, &srcLocation, nullptr);

		// 转换资源状态
		D3D12_RESOURCE_BARRIER barrier = {};
		barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
		barrier.Flags = D3D12_RESOURCE_BARRIER_FLAG_NONE;
		barrier.Transition.pResource = m_textures[textureIndex].Get();
		barrier.Transition.StateBefore = D3D12_RESOURCE_STATE_COPY_DEST;
		barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
		barrier.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;

		commandList->ResourceBarrier(1, &barrier);

		// 创建SRV
		D3D12_SHADER_RESOURCE_VIEW_DESC srvDesc = {};
		srvDesc.Shader4ComponentMapping = D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING;
		srvDesc.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
		srvDesc.ViewDimension = D3D12_SRV_DIMENSION_TEXTURE2D;
		srvDesc.Texture2D.MipLevels = 1;
		srvDesc.Texture2D.MostDetailedMip = 0;
		srvDesc.Texture2D.PlaneSlice = 0;
		srvDesc.Texture2D.ResourceMinLODClamp = 0.0f;

		// 获取SRV堆中的句柄
		ID3D12DescriptorHeap* srvHeap = m_renderer->GetSRVDescriptorHeap();
		UINT descriptorSize = device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV);

		D3D12_CPU_DESCRIPTOR_HANDLE cpuHandle = srvHeap->GetCPUDescriptorHandleForHeapStart();
		cpuHandle.ptr += textureIndex * descriptorSize;

		device->CreateShaderResourceView(m_textures[textureIndex].Get(), &srvDesc, cpuHandle);

		// 保存GPU句柄
		D3D12_GPU_DESCRIPTOR_HANDLE gpuHandle = srvHeap->GetGPUDescriptorHandleForHeapStart();
		gpuHandle.ptr += textureIndex * descriptorSize;
		m_textureSrvHandles[textureIndex] = gpuHandle;

		// 将纹理 SRV 句柄传递给 D3D12Renderer
		m_renderer->SetTextureSrvHandle(textureIndex, gpuHandle);

		return true;
	}
    void Live2DModelDX12::CreateVertexIndexBuffers()
    	{
    		if (!m_model)
    		{
    			return;
    		}
    
    		ID3D12Device* device = m_renderer->GetDevice();
    
    		// 为每个可绘制对象创建顶点和索引缓冲区
    		m_vertexBuffers.resize(m_drawableCount);
    		m_indexBuffers.resize(m_drawableCount);
    
    		for (int i = 0; i < m_drawableCount; i++)
    		{
    			int vertexCount = m_model->GetDrawableVertexCount(i);
    			int indexCount = m_model->GetDrawableVertexIndexCount(i);
    
    			if (vertexCount == 0 || indexCount == 0)
    			{
    				continue;
    			}
    
    			// 创建顶点缓冲区
    			D3D12_HEAP_PROPERTIES heapProps = {};
    			heapProps.Type = D3D12_HEAP_TYPE_UPLOAD;
    			heapProps.CreationNodeMask = 1;
    			heapProps.VisibleNodeMask = 1;
    
    			D3D12_RESOURCE_DESC vertexBufferDesc = {};
    			vertexBufferDesc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    			vertexBufferDesc.Alignment = 0;
    			vertexBufferDesc.Width = vertexCount * sizeof(CubismVertex);
    			vertexBufferDesc.Height = 1;
    			vertexBufferDesc.DepthOrArraySize = 1;
    			vertexBufferDesc.MipLevels = 1;
    			vertexBufferDesc.Format = DXGI_FORMAT_UNKNOWN;
    			vertexBufferDesc.SampleDesc.Count = 1;
    			vertexBufferDesc.SampleDesc.Quality = 0;
    			vertexBufferDesc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    			vertexBufferDesc.Flags = D3D12_RESOURCE_FLAG_NONE;
    
    			HRESULT hr = device->CreateCommittedResource(
    				&heapProps,
    				D3D12_HEAP_FLAG_NONE,
    				&vertexBufferDesc,
    				D3D12_RESOURCE_STATE_GENERIC_READ,
    				nullptr,
    				IID_PPV_ARGS(&m_vertexBuffers[i])
    			);
    
    			if (FAILED(hr))
    			{
    				continue;
    			}
    
    			// 创建索引缓冲区
    			D3D12_RESOURCE_DESC indexBufferDesc = {};
    			indexBufferDesc.Dimension = D3D12_RESOURCE_DIMENSION_BUFFER;
    			indexBufferDesc.Alignment = 0;
    			indexBufferDesc.Width = indexCount * sizeof(csmUint16);
    			indexBufferDesc.Height = 1;
    			indexBufferDesc.DepthOrArraySize = 1;
    			indexBufferDesc.MipLevels = 1;
    			indexBufferDesc.Format = DXGI_FORMAT_UNKNOWN;
    			indexBufferDesc.SampleDesc.Count = 1;
    			indexBufferDesc.SampleDesc.Quality = 0;
    			indexBufferDesc.Layout = D3D12_TEXTURE_LAYOUT_ROW_MAJOR;
    			indexBufferDesc.Flags = D3D12_RESOURCE_FLAG_NONE;
    
    			hr = device->CreateCommittedResource(
    				&heapProps,
    				D3D12_HEAP_FLAG_NONE,
    				&indexBufferDesc,
    				D3D12_RESOURCE_STATE_GENERIC_READ,
    				nullptr,
    				IID_PPV_ARGS(&m_indexBuffers[i])
    			);
    
    			if (FAILED(hr))
    			{
    				continue;
    			}
    
    			// 将缓冲区传递给 D3D12Renderer
    			m_renderer->SetVertexBuffer(i, m_vertexBuffers[i].Get());
    			m_renderer->SetIndexBuffer(i, m_indexBuffers[i].Get());

    			// 填充索引缓冲区数据
    			const csmUint16* indices = m_model->GetDrawableVertexIndices(i);
    			if (indices)
    			{
    				void* pIndexData = nullptr;
    				D3D12_RANGE readRange = { 0, 0 };
    				HRESULT hr = m_indexBuffers[i]->Map(0, &readRange, &pIndexData);
    				if (SUCCEEDED(hr))
    				{
    					memcpy(pIndexData, indices, indexCount * sizeof(csmUint16));
    					m_indexBuffers[i]->Unmap(0, nullptr);
    				}
    			}
    		}
    	}
    void Live2DModelDX12::UpdateVertexBuffer()
    {
        if (!m_model)
        {
            return;
        }

        // 更新所有可绘制对象的顶点缓冲区
        for (int i = 0; i < m_drawableCount; i++)
        {
            int vertexCount = m_model->GetDrawableVertexCount(i);
            if (vertexCount == 0)
            {
                continue;
            }

            // 获取顶点数据
            const csmFloat32* vertices = m_model->GetDrawableVertices(i);
            const csmFloat32* uvs = m_model->GetDrawableVertexUvs(i);

            // 创建顶点数据数组
            std::vector<CubismVertex> vertexData(vertexCount);

            for (int j = 0; j < vertexCount; j++)
            {
                vertexData[j].position[0] = vertices[j * 2];
                vertexData[j].position[1] = vertices[j * 2 + 1];
                vertexData[j].uv[0] = uvs[j * 2];
                vertexData[j].uv[1] = uvs[j * 2 + 1];
            }

            // 映射顶点缓冲区并更新数据
            void* pData = nullptr;
            D3D12_RANGE readRange = { 0, 0 };

            HRESULT hr = m_vertexBuffers[i]->Map(0, &readRange, &pData);
            if (SUCCEEDED(hr))
            {
                memcpy(pData, vertexData.data(), vertexCount * sizeof(CubismVertex));
                m_vertexBuffers[i]->Unmap(0, nullptr);
            }
        }
    }

    void Live2DModelDX12::Update(float deltaTime)
    {
        if (!m_model)
        {
            return;
        }

        // 更新模型
        m_model->Update();
        m_model->GetModel()->SetOpacity(1.0f);

        // 更新顶点缓冲区
        UpdateVertexBuffer();
    }

    void Live2DModelDX12::Render()
    {
        if (!m_model || !m_cubismRenderer)
        {
            return;
        }

        // 渲染剪裁遮罩
        RenderClipMasks();

        // 渲染模型
        m_cubismRenderer->DrawModel();
    }

    void Live2DModelDX12::RenderClipMasks()
    	{
    		if (!m_model || !m_cubismRenderer)
    		{
    			return;
    		}
    
    		// 检查是否有剪裁遮罩
    		int maskCount = m_model->GetDrawableClipCount();
    		if (maskCount == 0)
    		{
    			return;
    		}
    
    		// 注意：完整的剪裁遮罩渲染需要更复杂的实现
    		// 这里暂时简化处理，只进行基本的剪裁检查
    		// 实际应用中需要实现多通道渲染来正确处理剪裁遮罩
    
    		m_cubismRenderer->PreDraw();
    
    		for (int i = 0; i < m_drawableCount; i++)
    		{
    			// 检查是否需要剪裁
    			const csmInt32* clipMasks = m_model->GetDrawableDrawableMasks(i);
    			int clipMaskCount = m_model->GetDrawableDrawableMaskCounts(i);
    
    			if (clipMaskCount > 0)
    			{
    				// 暂时跳过剪裁遮罩的渲染
    				// 完整实现需要创建遮罩纹理和离屏渲染
    			}
    		}
    
    		m_cubismRenderer->PostDraw();
    	}
    void Live2DModelDX12::DrawDrawable(int drawableIndex)
    {
        if (!m_model || !m_cubismRenderer || drawableIndex < 0 || drawableIndex >= m_drawableCount)
        {
            return;
        }

        // 检查可绘制对象是否可见
        if (!m_model->GetDrawableDynamicFlagIsVisible(drawableIndex))
        {
            return;
        }

        // 渲染可绘制对象
        m_cubismRenderer->DrawMesh(drawableIndex);
    }

    void Live2DModelDX12::SetParameter(const char* id, float value)
    {
        if (!m_model)
        {
            return;
        }

        // 查找参数ID
        CubismIdHandle parameterId = m_model->GetParameterId(id);
        if (parameterId)
        {
            m_model->SetParameterValue(parameterId, value);
        }
    }
}