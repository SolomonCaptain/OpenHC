#pragma once
#include <d3d12.h>
#include <dxgi1_6.h>
#include <dxgi1_4.h>
#include <wrl/client.h>
#include <winrt/Windows.UI.Xaml.Controls.h>

using Microsoft::WRL::ComPtr;

namespace Live2D_Native
{
	class D3D12Renderer
	{
	public:
		D3D12Renderer();
		~D3D12Renderer();

		HRESULT Initialize(winrt::Windows::UI::Xaml::Controls::SwapChainPanel const& panel);
		void Resize(UINT width, UINT height);
		void Present();
		void WaitForGPU();

		// 命令列表管理
		HRESULT OpenCommandList(); // 在渲染前调用
		HRESULT CloseCommandList(); // 在渲染完成后调用

		// 获取设备、命令队列等对象供其他类使用
		ID3D12Device* GetDevice() const { return m_device.Get(); }
		ID3D12CommandQueue* GetCommandQueue() const { return m_commandQueue.Get(); }
		ID3D12GraphicsCommandList* GetCommandList() const { return m_commandList.Get(); }
		ID3D12DescriptorHeap* GetSRVDescriptionHeap() const { return m_srvHeap.Get(); }
		ID3D12DescriptorHeap* GetRTVHeap() const { return m_rtvHeap.Get(); }
		UINT GetRTVDescriptorSize() const { return m_rtvDescriptorSize; }
		DXGI_FORMAT GetBackBufferFormat() const { return m_backBufferFormat; }
		UINT GetFrameIndex() const { return m_frameIndex; }
		UINT GetWidth() const { return m_width; }
		UINT GetHeight() const { return m_height; }
		ID3D12Resource* GetRenderTarget(int index) const { return m_renderTargets[index].Get(); }

		// 获取顶点和索引缓冲区（由 Live2DModelDX12 管理）
		ID3D12Resource* GetVertexBuffer(int index) const;
		ID3D12Resource* GetIndexBuffer(int index) const;
		D3D12_GPU_DESCRIPTOR_HANDLE GetTextureSrvHandle(int index) const;

		// 设置顶点和索引缓冲区（由 Live2DModelDX12 调用）
		void SetVertexBuffer(int index, ID3D12Resource* buffer);
		void SetIndexBuffer(int index, ID3D12Resource* buffer);
		void SetTextureSrvHandle(int index, D3D12_GPU_DESCRIPTOR_HANDLE handle);

	private:
		static const UINT FrameCount = 2;

		// DX12 核心对象
		ComPtr<ID3D12Device> m_device;
		ComPtr<ID3D12CommandQueue> m_commandQueue;
		ComPtr<IDXGISwapChain3> m_swapChain;
		ComPtr<IDXGIFactory4> m_factory;
		ComPtr<ID3D12DescriptorHeap> m_rtvHeap;
		ComPtr<ID3D12DescriptorHeap> m_srvHeap;
		ComPtr<ID3D12CommandAllocator> m_commandAllocator;
		ComPtr<ID3D12GraphicsCommandList> m_commandList;

		// 渲染目标视图
		ComPtr<ID3D12Resource> m_renderTargets[FrameCount];
		UINT m_rtvDescriptorSize;

		// 同步对象
		ComPtr<ID3D12Fence> m_fence;
		UINT64 m_fenceValue;
		HANDLE m_fenceEvent;

		// 状态变量
		UINT m_frameIndex;
		DXGI_FORMAT m_backBufferFormat;
		UINT m_width;
		UINT m_height;

		// SwapChainPanel 引用
		winrt::Windows::UI::Xaml::Controls::SwapChainPanel m_panel;

		// 顶点和索引缓冲区（由 Live2DModelDX12 管理）
		std::vector<ComPtr<ID3D12Resource>> m_vertexBuffers;
		std::vector<ComPtr<ID3D12Resource>> m_indexBuffers;
		D3D12_GPU_DESCRIPTOR_HANDLE m_textureSrvHandles[64];

		HRESULT CreateDeviceResources();
		HRESULT CreateWindowResources();
		void MoveToNextFrame();
	};
}