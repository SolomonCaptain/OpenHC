#pragma once
#include "Live2DModel.g.h"
#include <winrt/Windows.UI.Xaml.Controls.h>
#include "D3D12Renderer.h"
#include "Live2DModelDX12.h"

namespace winrt::Live2DWinRT::implementation
{
    struct Live2DModel : Live2DModelT<Live2DModel>
    {
        Live2DModel() = default;

        void Initialize(Windows::UI::Xaml::Controls::SwapChainPanel const& panel);
        void LoadModel(hstring const& moc3Path, hstring const& model3JsonPath, hstring const& textureDir);
        void Update(float deltaTime);
        void Render();
        void SetParameter(hstring const& id, float value);

        // 事件处理
        winrt::event_token ModelLoaded(Windows::Foundation::TypedEventHandler<Live2DModel, Windows::Foundation::IInspectable> const& handler);
        void ModelLoaded(winrt::event_token const& token) noexcept;

    private:
        std::unique_ptr<Live2D_Native::D3D12Renderer> m_renderer;
        std::unique_ptr<Live2D_Native::Live2DModelDX12> m_model;
        winrt::Windows::UI::Xaml::Controls::SwapChainPanel m_panel{ nullptr };

        // 事件
        winrt::event<Windows::Foundation::TypedEventHandler<Live2DModel, Windows::Foundation::IInspectable>> m_modelLoadedEvent;
    };
}

namespace winrt::Live2DWinRT::factory_implementation
{
    struct Live2DModel : Live2DModelT<Live2DModel, implementation::Live2DModel>
    {
    };
}