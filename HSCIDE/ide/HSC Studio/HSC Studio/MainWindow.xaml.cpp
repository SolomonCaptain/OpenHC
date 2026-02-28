#include "pch.h"
#include "MainWindow.xaml.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif

#include <winrt/Windows.Storage.h>
#include <winrt/Windows.Storage.Pickers.h>
#include <winrt/Windows.Storage.Streams.h>
#include <shobjidl.h>  // IInitializeWithWindow

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Windows::UI::Xaml::Controls;

namespace winrt::HSC_Studio::implementation
{
    MainWindow::MainWindow()
    {
        InitializeComponent();

        // 获取 SwapChainPanel
        m_panel = this->Content().try_as<Panel>().Children().GetAt(1).try_as<SwapChainPanel>();
    }

    void MainWindow::OnLoaded(IInspectable const&, RoutedEventArgs const&)
    {
        // 初始化 Live2D 模型
        m_model = winrt::make<winrt::Live2DWinRT::implementation::Live2DModel>();

        // 监听模型加载完成事件
        winrt::event_token modelLoadedToken = m_model.ModelLoaded([this](auto&& sender, auto&& args)
        {
            m_isModelLoaded = true;
            m_lastTime = std::chrono::steady_clock::now();

            // 开始渲染循环
            auto compositionTarget = Windows::UI::Xaml::Media::CompositionTarget::GetForCurrentView();
            m_renderingToken = compositionTarget.Rendering({ this, &MainWindow::OnCompositionRendering });
        });
    }

    void MainWindow::OnLoadModel(IInspectable const&, RoutedEventArgs const&)
    {
        // 打开文件选择器
        Windows::Storage::Pickers::FileOpenPicker picker;
        picker.SuggestedStartLocation(Windows::Storage::Pickers::PickerLocationId::Desktop);
        picker.FileTypeFilter().Append(L".json");
        picker.FileTypeFilter().Append(L".model3.json");

        auto hwnd = reinterpret_cast<HWND>(this->InteropService().WindowHandle());
        picker.as<IInitializeWithWindow>()->Initialize(hwnd);

        picker.PickSingleFileAsync().Completed([this](auto&& sender, auto&& args)
        {
            if (args.Status() == AsyncStatus::Completed)
            {
                auto model3File = args.GetResults();
                if (model3File)
                {
                    // 获取模型目录
                    auto model3Path = model3File.Path();

                    // 构建路径
                    hstring modelDir = model3Path;
                    size_t lastSlash = modelDir.find_last_of(L'\\');
                    if (lastSlash != hstring::npos)
                    {
                        modelDir = modelDir.substr(0, lastSlash);
                    }

                    // 初始化渲染器
                    m_model.Initialize(m_panel);

                    // 加载模型
                    m_model.LoadModel(
                        L"",  // moc3Path - 从 model3.json 中获取
                        model3Path,
                        modelDir
                    );
                }
            }
        });
    }

    void MainWindow::OnChangeParam(IInspectable const&, RoutedEventArgs const&)
    {
        if (!m_isModelLoaded || !m_model)
        {
            return;
        }

        // 示例：改变参数值
        m_model.SetParameter(L"ParamAngleX", 30.0f);
    }

    void MainWindow::OnResetParam(IInspectable const&, RoutedEventArgs const&)
    {
        if (!m_isModelLoaded || !m_model)
        {
            return;
        }

        // 示例：重置参数值
        m_model.SetParameter(L"ParamAngleX", 0.0f);
    }

    void MainWindow::OnCompositionRendering(IInspectable const&, IInspectable const&)
    {
        if (!m_isModelLoaded || !m_model)
        {
            return;
        }

        // 计算 deltaTime
        auto currentTime = std::chrono::steady_clock::now();
        float deltaTime = std::chrono::duration<float>(currentTime - m_lastTime).count();
        m_lastTime = currentTime;

        // 限制 deltaTime 防止过大
        if (deltaTime > 0.1f)
        {
            deltaTime = 0.1f;
        }

        // 更新和渲染模型
        UpdateModel(deltaTime);
        RenderModel();
    }

    void MainWindow::OnUnloaded(IInspectable const&, RoutedEventArgs const&)
    {
        // 停止渲染循环
        if (m_renderingToken)
        {
            auto compositionTarget = Windows::UI::Xaml::Media::CompositionTarget::GetForCurrentView();
            compositionTarget.Rendering(m_renderingToken);
        }

        // 清理资源
        m_model = nullptr;
        m_panel = nullptr;
    }

    void MainWindow::UpdateModel(float deltaTime)
    {
        if (m_model)
        {
            m_model.Update(deltaTime);
        }
    }

    void MainWindow::RenderModel()
    {
        if (m_model)
        {
            m_model.Render();
        }
    }
}
