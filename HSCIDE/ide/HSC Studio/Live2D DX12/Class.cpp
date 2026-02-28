#include "pch.h"
#include "Class.h"
#include "module.g.cpp"

using namespace winrt;
using namespace Windows::UI::Xaml::Controls;

namespace winrt::Live2DWinRT::implementation
{
    void Live2DModel::Initialize(SwapChainPanel const& panel)
    {
        m_panel = panel;

        // 创建Native渲染器
        m_renderer = std::make_unique<Live2D_Native::D3D12Renderer>();

        // 初始化渲染器
        HRESULT hr = m_renderer->Initialize(panel);
        if (FAILED(hr))
        {
            throw hresult_error(E_FAIL, L"Failed to initialize D3D12Renderer");
        }

        // 监听面板大小变化
        m_panel.SizeChanged([this](IInspectable const&, SizeChangedEventArgs const& e)
            {
                if (m_renderer)
                {
                    m_renderer->Resize(
                        static_cast<UINT>(e.NewSize().Width),
                        static_cast<UINT>(e.NewSize().Height)
                    );
                }
            });
    }

    void Live2DModel::LoadModel(hstring const& moc3Path, hstring const& model3JsonPath, hstring const& textureDir)
    {
        if (!m_renderer)
        {
            throw hresult_error(E_INVALIDARG, L"Renderer not initialized");
        }

        // 创建模型
        m_model = std::make_unique<Live2D_Native::Live2DModelDX12>(m_renderer.get());

        // 加载模型
        bool success = m_model->LoadModel(
            to_string(moc3Path).c_str(),
            to_string(model3JsonPath).c_str(),
            to_string(textureDir).c_str()
        );

        if (!success)
        {
            throw hresult_error(E_FAIL, L"Failed to load Live2D model");
        }

        // 触发模型加载完成事件
        m_modelLoadedEvent(*this, nullptr);
    }

    void Live2DModel::Update(float deltaTime)
    {
        if (m_model)
        {
            m_model->Update(deltaTime);
        }
    }

    void Live2DModel::Render()
    {
        if (m_model)
        {
            m_model->Render();
        }

        if (m_renderer)
        {
            m_renderer->Present();
        }
    }

    void Live2DModel::SetParameter(hstring const& id, float value)
    {
        if (m_model)
        {
            m_model->SetParameter(to_string(id).c_str(), value);
        }
    }

    winrt::event_token Live2DModel::ModelLoaded(Windows::Foundation::TypedEventHandler<Live2DModel, Windows::Foundation::IInspectable> const& handler)
    {
        return m_modelLoadedEvent.add(handler);
    }

    void Live2DModel::ModelLoaded(winrt::event_token const& token) noexcept
    {
        m_modelLoadedEvent.remove(token);
    }
}