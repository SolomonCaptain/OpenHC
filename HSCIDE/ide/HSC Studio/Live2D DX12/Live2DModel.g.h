// 计划（伪代码，逐步详细说明）：
// 1. 问题：编译器报错无法打开生成的头文件 "Live2DModel.g.h"。
// 2. 目的：在缺少 XAML/codegen 自动生成文件时提供一个最小的占位头文件，
//    使现有代码（Class.h）能够正常编译，直到正确生成的 .g.h 文件可用。
// 3. 实现细节：
//    - 提供包含保护宏，避免重复包含。
//    - 在命名空间 `winrt::Live2DWinRT` 中定义模板 `Live2DModelT`，
//      该模板带有两个模板参数（第二个有默认类型），以匹配项目中两种使用方式：
//        Live2DModelT<Live2DModel>
//        Live2DModelT<Live2DModel, implementation::Live2DModel>
//    - 该模板为空（占位），保证继承与声明在编译期存在，不改变运行逻辑。
//    - 在文件头部添加说明注释，提醒这是临时替代，真正的 `Live2DModel.g.h` 应由 XAML/codegen 生成。
// 4. 将此文件添加到工程中（与其他源文件同目录），以解决 E1696 错误。
// 5. 后续：启用/修复 XAML 代码生成器或移除此占位文件以使用真实的生成文件。

#ifndef LIVE2DMODEL_G_H
#define LIVE2DMODEL_G_H

// 临时占位的生成头文件（由 XAML/codegen 生成的真实文件应替换此文件）。
// 注意：这个文件只提供最小的声明以解决编译时缺失引用问题。

namespace winrt::Live2DWinRT
{
    // 提供两个模板参数（第二个有默认值），以兼容多种使用形式。
    template <typename D, typename I = void>
    struct Live2DModelT {};
}

#endif // LIVE2DMODEL_G_H