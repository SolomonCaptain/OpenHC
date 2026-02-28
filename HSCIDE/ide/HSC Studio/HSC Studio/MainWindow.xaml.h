#pragma once
#include "MainWindow.g.h"
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <chrono>

namespace winrt::HSC_Studio::implementation
{
    struct MainWindow : MainWindowT<MainWindow>
    {
        MainWindow();
        void OnLoaded(IInspectable const&, RoutedEventArgs const&);
        void OnLoadModel(IInspectable const&, RoutedEventArgs const&);
        void OnChangeParam(IInspectable const&, RoutedEventArgs const&);
        void OnResetParam(IInspectable const&, RoutedEventArgs const&);
        void OnCompositionRendering(IInspectable const&, IInspectable const&);
        void OnUnloaded(IInspectable const&, RoutedEventArgs const&);

    private:
        winrt::Live2DWinRT::Live2DModel m_model{ nullptr };
        winrt::Windows::UI::Xaml::Controls::SwapChainPanel m_panel{ nullptr };
        winrt::event_token m_renderingToken;
        std::chrono::steady_clock::time_point m_lastTime;
        bool m_isModelLoaded{ false };

        void UpdateModel(float deltaTime);
        void RenderModel();
    };
}

namespace winrt::HSC_Studio::factory_implementation
{
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow>
    {
    };
}
