#include "pch.h"
#include "D3D12Renderer.h"
#include <d3d12sdklayers.h>
#include <Windows.Graphics.DirectX.Direct3D11.interop.h>
#include <Windows.Graphics.DirectX.Direct3D12.interop.h>
#include <d3dx12.h>

using namespace winrt::Windows::UI::Xaml::Controls;

namespace Live2D_Native
{
	D3D12Renderer::D3D12Renderer()
		: m_frameIndex(0)
		, m_rtvDescriptorSize(0)
		, m_fenceValue(0)
		, m_fenceEvent(nullptr)
		, m_backBufferFormat(DXGI_FORMAT_R8G8B8A8_UNORM)
		, m_width(0)
		, m_height(0)
	{
	}

	D3D12Renderer::~D3D12Renderer()
	{
		// 确保 GPU 已完成所有工作
		WaitForGPU();

		if (m_fenceEvent)
		{
			CloseHandle(m_fenceEvent);
			m_fenceEvent = nullptr;
		}
	}

	HRESULT D3D12Renderer::Initialize(SwapChainPanel const& panel)
	{
		m_panel = panel;

		// 获取面板尺寸
		m_width = static_cast<UINT>(panel.ActualWidth());
		m_height = static_cast<UINT>(panel.ActualHeight());

		// 创建 DX12 设备资源
		HRESULT hr = CreateDeviceResources();
		if (FAILED(hr))
		{
			return hr;
		}

		// 创建窗口相关资源（交换链等）
		hr = CreateWindowResources();
		if (FAILED(hr))
		{
			return hr;
		}

		return S_OK;
	}

	HRESULT D3D12Renderer::CreateDeviceResources()
	{
		HRESULT hr;

		// 启用调试层（仅限调试模式）
#if defined(_DEBUG)
		ComPtr<ID3D12Debug> debugController;
		if (SUCCEEDED(D3D12GetDebugInterface(IID_PPV_ARGS(&debugController))))
		{
			debugController->EnableDebugLayer();
		}
#endif

		// 创建 DX12 设备
		hr = D3D12CreateDevice(
			nullptr,					// 默认适配器
			D3D_FEATURE_LEVEL_11_0,		// 最低功能级别
			IID_PPV_ARGS(&m_device)
		);

		if (FAILED(hr))
		{
			return hr;
		}

		// 创建命令队列
		D3D12_COMMAND_QUEUE_DESC queueDesc = {};
		queueDesc.Flags = D3D12_COMMAND_QUEUE_FLAG_NONE;
		queueDesc.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;

		hr = m_device->CreateCommandQueue(&queueDesc, IID_PPV_ARGS(&m_commandQueue));
		if (FAILED(hr))
		{
			return hr;
		}

		// 创建命令分配器
		hr = m_device->CreateCommandAllocator(
			D3D12_COMMAND_LIST_TYPE_DIRECT,
			IID_PPV_ARGS(&m_commandAllocator)
		);
		if (FAILED(hr))
		{
			return hr;
		}

		// 创建命令列表
		hr = m_device->CreateCommandList(
			0,
			D3D12_COMMAND_LIST_TYPE_DIRECT,
			m_commandAllocator.Get(),
			nullptr,
			IID_PPV_ARGS(&m_commandList)
		);
		if (FAILED(hr))
		{
			return hr;
		}

		// 命令列表创建时处于记录状态，需要关闭
		m_commandList->Close();

		// 创建 RTV 描述符堆
		D3D12_DESCRIPTOR_HEAP_DESC rtvHeapDesc = {};
		rtvHeapDesc.NumDescriptors = FrameCount;
		rtvHeapDesc.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
		rtvHeapDesc.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_NONE;

		hr = m_device->CreateDescriptorHeap(&rtvHeapDesc, IID_PPV_ARGS(&m_rtvHeap));
		if (FAILED(hr))
		{
			return hr;
		}

		m_rtvDescriptorSize = m_device->GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);

		// 创建 SRV 描述符堆（用于纹理）
		D3D12_DESCRIPTOR_HEAP_DESC srvHeapDesc = {};
		srvHeapDesc.NumDescriptors = 1024; // 根据需要调整
		srvHeapDesc.Type = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV;
		srvHeapDesc.Flags = D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE;

		hr = m_device->CreateDescriptorHeap(&srvHeapDesc, IID_PPV_ARGS(&m_srvHeap));
		if (FAILED(hr))
		{
			return hr;
		}

		// 创建围栏
		hr = m_device->CreateFence(0, D3D12_FENCE_FLAG_NONE, IID_PPV_ARGS(&m_fence));
		if (FAILED(hr))
		{
			return hr;
		}

		m_fenceValue = 1;

		// 创建围栏事件
		m_fenceEvent = CreateEvent(nullptr, FALSE, FALSE, nullptr);
		if (m_fenceEvent == nullptr)
		{
			return HRESULT_FROM_WIN32(GetLastError());
		}

		return S_OK;
	}

	HRESULT D3D12Renderer::CreateWindowResources()
	{
		HRESULT hr;

		// 清除之前的渲染目标
		for (UINT n = 0; n < FrameCount; n++)
		{
			m_renderTargets[n].Reset();
		}

		// 获取 SwapChainPanel 的原生接口
		auto panelNative = m_panel.as<ISwapChainPanelNative>();

		// 创建交换链描述
		DXGI_SWAP_CHAIN_DESC1 swapChainDesc = {};
		swapChainDesc.Width = m_width;
		swapChainDesc.Height = m_height;
		swapChainDesc.Format = m_backBufferFormat;
		swapChainDesc.Stereo = FALSE;
		swapChainDesc.SampleDesc.Count = 1;
		swapChainDesc.SampleDesc.Quality = 0;
		swapChainDesc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
		swapChainDesc.BufferCount = FrameCount;
		swapChainDesc.Scaling = DXGI_SCALING_STRETCH;
		swapChainDesc.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
		swapChainDesc.AlphaMode = DXGI_ALPHA_MODE_PREMULTIPLIED;
		swapChainDesc.Flags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;

		// 创建交换链
		ComPtr<IDXGISwapChain1> swapChain;
		hr = CreateDXGIFactory1(IID_PPV_ARGS(&m_factory));
		if (FAILED(hr))
		{
			return hr;
		}

		hr = m_factory->CreateSwapChainForComposition(
			m_commandQueue.Get(),
			&swapChainDesc,
			nullptr,
			&swapChain
		);
		if (FAILED(hr))
		{
			return hr;
		}

		// 获取 IDXGISwapChain3 接口
		hr = swapChain.As(&m_swapChain);
		if (FAILED(hr))
		{
			return hr;
		}

		// 将交换链与 SwapChainPanel 关联
		hr = panelNative->SetSwapChain(m_swapChain.Get());
		if (FAILED(hr))
		{
			return hr;
		}

		// 创建渲染目标视图
		CD3DX12_CPU_DESCRIPTOR_HANDLE rtvHandle(m_rtvHeap->GetCPUDescriptorHandleForHeapStart());
		for (UINT n = 0; n < FrameCount; n++)
		{
			hr = m_swapChain->GetBuffer(n, IID_PPV_ARGS(&m_renderTargets[n]));
			if (FAILED(hr))
			{
				return hr;
			}

			m_device->CreateRenderTargetView(m_renderTargets[n].Get(), nullptr, rtvHandle);
			rtvHandle.Offset(1, m_rtvDescriptorSize);
		}

		return S_OK;
	}

	HRESULT D3D12Renderer::OpenCommandList()
	{
		HRESULT hr = m_commandAllocator->Reset();
		if (FAILED(hr))
		{
			return hr;
		}

		hr = m_commandList->Reset(m_commandAllocator.Get(), nullptr);
		if (FAILED(hr))
		{
			return hr;
		}

		return S_OK;
	}

	HRESULT D3D12Renderer::CloseCommandList()
	{
		return m_commandList->Close();
	}

	void D3D12Renderer::Resize(UINT width, UINT height)
	{
		if (width == 0 || height == 0)
		{
			return;
		}

		// 等待 GPU 完成当前帧的工作
		WaitForGPU();

		// 释放当前的渲染目标
		for (UINT n = 0; n < FrameCount; n++)
		{
			m_renderTargets[n].Reset();
		}

		// 调整交换链大小
		DXGI_SWAP_CHAIN_DESC desc = {};
		m_swapChain->GetDesc(&desc);

		HRESULT hr = m_swapChain->ResizeBuffers(
			FrameCount,
			width,
			height,
			desc.BufferDesc.Format,
			desc.Flags
		);

		if (FAILED(hr))
		{
			return;
		}

		// 重新创建渲染目标视图
		CD3DX12_CPU_DESCRIPTOR_HANDLE rtvHandle(m_rtvHeap->GetCPUDescriptorHandleForHeapStart());
		for (UINT n = 0; n < FrameCount; n++)
		{
			hr = m_swapChain->GetBuffer(n, IID_PPV_ARGS(&m_renderTargets[n]));
			if (FAILED(hr))
			{
				return;
			}

			m_device->CreateRenderTargetView(m_renderTargets[n].Get(), nullptr, rtvHandle);
			rtvHandle.Offset(1, m_rtvDescriptorSize);
		}

		m_frameIndex = m_swapChain->GetCurrentBackBufferIndex();
	}

	void D3D12Renderer::Present()
	    {
	        // 执行命令列表
	        ID3D12CommandList* ppCommandLists[] = { m_commandList.Get() };
	        m_commandQueue->ExecuteCommandLists(_countof(ppCommandLists), ppCommandLists);
	
	        // 显示交换链
	        HRESULT hr = m_swapChain->Present(1, 0);
	        if (FAILED(hr))
	        {
	            return;
	        }
	
	        // 移动到下一帧
	        MoveToNextFrame();
	    }
	void D3D12Renderer::WaitForGPU()
	{
		// 等待信号操作
		const UINT64 fence = m_fenceValue;
		HRESULT hr = m_commandQueue->Signal(m_fence.Get(), fence);
		if (FAILED(hr))
		{
			return;
		}

		m_fenceValue++;

		// 等待 GPU 到达围栏
		if (m_fence->GetCompletedValue() < fence)
		{
			hr = m_fence->SetEventOnCompletion(fence, m_fenceEvent);
			if (SUCCEEDED(hr))
			{
				WaitForSingleObject(m_fenceEvent, INFINITE);
			}
		}
	}

	void D3D12Renderer::MoveToNextFrame()
	{
		// 等待信号操作
		const UINT64 currentFenceValue = m_fenceValue;
		HRESULT hr = m_commandQueue->Signal(m_fence.Get(), currentFenceValue);
		if (FAILED(hr))
		{
			return;
		}

		m_fenceValue++;

		// 更新帧索引
		m_frameIndex = m_swapChain->GetCurrentBackBufferIndex();

		// 如果 GPU 已经完成下一帧，则不需要等待
		if (m_fence->GetCompletedValue() < currentFenceValue)
		{
			hr = m_fence->SetEventOnCompletion(currentFenceValue, m_fenceEvent);
			if (SUCCEEDED(hr))
			{
				WaitForSingleObject(m_fenceEvent, INFINITE);
			}
		}
	}

	ID3D12Resource* D3D12Renderer::GetVertexBuffer(int index) const
	{
		if (index >= 0 && index < static_cast<int>(m_vertexBuffers.size()))
		{
			return m_vertexBuffers[index].Get();
		}
		return nullptr;
	}

	ID3D12Resource* D3D12Renderer::GetIndexBuffer(int index) const
	{
		if (index >= 0 && index < static_cast<int>(m_indexBuffers.size()))
		{
			return m_indexBuffers[index].Get();
		}
		return nullptr;
	}

	D3D12_GPU_DESCRIPTOR_HANDLE D3D12Renderer::GetTextureSrvHandle(int index) const
	{
		if (index >= 0 && index < 64)
		{
			return m_textureSrvHandles[index];
		}
		return D3D12_GPU_DESCRIPTOR_HANDLE();
	}

	void D3D12Renderer::SetVertexBuffer(int index, ID3D12Resource* buffer)
	{
		if (index >= static_cast<int>(m_vertexBuffers.size()))
		{
			m_vertexBuffers.resize(index + 1);
		}
		m_vertexBuffers[index] = buffer;
	}

	void D3D12Renderer::SetIndexBuffer(int index, ID3D12Resource* buffer)
	{
		if (index >= static_cast<int>(m_indexBuffers.size()))
		{
			m_indexBuffers.resize(index + 1);
		}
		m_indexBuffers[index] = buffer;
	}

	void D3D12Renderer::SetTextureSrvHandle(int index, D3D12_GPU_DESCRIPTOR_HANDLE handle)
	{
		if (index >= 0 && index < 64)
		{
			m_textureSrvHandles[index] = handle;
		}
	}
}