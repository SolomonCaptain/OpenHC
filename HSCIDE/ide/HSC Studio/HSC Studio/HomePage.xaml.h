#pragma once
#include "HomePage.g.h"

namespace winrt::HSC_Studio::implementation
{
    struct HomePage : HomePageT<HomePage>
    {
        HomePage();
    };
}

namespace winrt::HSC_Studio::factory_implementation
{
    struct HomePage : HomePageT<HomePage, implementation::HomePage>
    {
    };
}