# Jalium.UI 开发者指南

本文档详细介绍如何使用 C# 和 JALXAML 协同开发 Jalium.UI 应用程序。

---

## 目录

1. [快速入门](#1-快速入门)
2. [应用程序架构](#2-应用程序架构)
3. [JALXAML 标记语言](#3-jalxaml-标记语言)
4. [布局系统](#4-布局系统)
5. [控件库](#5-控件库)
6. [数据绑定](#6-数据绑定)
7. [资源和样式](#7-资源和样式)
8. [控件模板](#8-控件模板)
9. [可视化状态管理](#9-可视化状态管理)
10. [路由事件](#10-路由事件)
11. [依赖属性](#11-依赖属性)
12. [MVVM 模式](#12-mvvm-模式)
13. [窗口和应用程序生命周期](#13-窗口和应用程序生命周期)
14. [主题系统](#14-主题系统)
15. [自定义控件开发](#15-自定义控件开发)
16. [高级主题](#16-高级主题)

---

## 1. 快速入门

### 1.1 安装

通过 NuGet 安装 Jalium.UI 元包：

```bash
dotnet add package Jalium.UI
```

或安装单独的包：

```bash
dotnet add package Jalium.UI.Core       # 核心类型
dotnet add package Jalium.UI.Controls   # 控件库
dotnet add package Jalium.UI.Xaml       # XAML 解析
dotnet add package Jalium.UI.Build      # 构建工具
```

### 1.2 创建第一个应用

**Program.cs**

```csharp
using Jalium.UI.Controls;

var app = new Application();

var window = new Window
{
    Title = "Hello Jalium.UI",
    Width = 960,
    Height = 640,
    Content = new StackPanel
    {
        Margin = new Thickness(24),
        Children =
        {
            new TextBlock { Text = "欢迎使用 Jalium.UI", FontSize = 28 },
            new TextBlock { Text = "GPU 加速的 .NET UI 框架", Margin = new Thickness(0, 8, 0, 16) },
            new Button { Content = "开始使用" }
        }
    }
};

app.Run(window);
```

### 1.3 使用 JALXAML

**MainWindow.jalxaml**

```xml
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        Title="我的第一个窗口"
        Width="800" Height="500">
    
    <Grid Margin="20">
        <StackPanel VerticalAlignment="Center" HorizontalAlignment="Center">
            <TextBlock Text="欢迎使用 Jalium.UI" 
                       FontSize="28" 
                       HorizontalAlignment="Center"/>
            <Button Content="点击开始" 
                    Margin="0,16,0,0" 
                    HorizontalAlignment="Center"
                    Click="OnStartClick"/>
        </StackPanel>
    </Grid>
</Window>
```

**MainWindow.jalxaml.cs (Code-Behind)**

```csharp
using Jalium.UI.Controls;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
    }

    private void OnStartClick(object sender, RoutedEventArgs e)
    {
        // 处理按钮点击
        MessageBox.Show("按钮被点击了！");
    }
}
```

**运行时解析 JALXAML**

```csharp
using Jalium.UI.Markup;

var xaml = """
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        Title="动态窗口" Width="400" Height="300">
    <TextBlock Text="运行时创建" HorizontalAlignment="Center" VerticalAlignment="Center"/>
</Window>
""";

var window = (Window)XamlReader.Parse(xaml);
window.Show();
```

---

## 2. 应用程序架构

### 2.1 应用程序入口

Jalium.UI 应用程序从 `Application` 类开始：

```csharp
public class Program
{
    [STAThread]
    public static void Main()
    {
        var app = new Application();
        
        // 方式 1：直接设置主窗口
        app.MainWindow = new MainWindow();
        app.Run();
        
        // 方式 2：使用 StartupUri
        // app.StartupUri = "MainWindow.jalxaml";
        // app.Run();
    }
}
```

### 2.2 应用程序生命周期

```csharp
public class MyApp : Application
{
    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        // 应用程序启动时的初始化
    }
    
    protected override void OnExit(ExitEventArgs e)
    {
        // 清理资源
        base.OnExit(e);
    }
}
```

### 2.3 应用程序资源

```csharp
var app = new Application();

// 添加全局资源
app.Resources["AccentColor"] = Color.FromRgb(0x00, 0x78, 0xD4);
app.Resources["AccentBrush"] = new SolidColorBrush(Color.FromRgb(0x00, 0x78, 0xD4));

// 合并资源字典
app.Resources.MergedDictionaries.Add(new ResourceDictionary
{
    Source = new Uri("Themes/Generic.jalxaml", UriKind.Relative)
});
```

---

## 3. JALXAML 标记语言

### 3.1 基本语法

JALXAML 是一种类似 XAML 的标记语言，用于声明式地定义用户界面。

**命名空间声明**

```xml
<!-- 默认命名空间 -->
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation">
    <!-- 控件 -->
</Window>

<!-- 完整命名空间声明 -->
<Window xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
        xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
        xmlns:local="clr-namespace:MyApp"
        xmlns:controls="clr-namespace:MyApp.Controls;assembly=MyApp.Controls">
</Window>
```

### 3.2 属性设置

**属性语法**

```xml
<Button Content="点击我" 
        Width="100" 
        Height="30" 
        Background="Blue"/>
```

**属性元素语法**

```xml
<Button>
    <Button.Content>
        <StackPanel Orientation="Horizontal">
            <Image Source="icon.png" Width="16" Height="16"/>
            <TextBlock Text="点击我" Margin="4,0,0,0"/>
        </StackPanel>
    </Button.Content>
</Button>
```

### 3.3 内容属性

某些控件支持直接设置内容：

```xml
<!-- Button 的 Content 是内容属性 -->
<Button>
    <TextBlock Text="按钮文本"/>
</Button>

<!-- Grid 的 Children 是内容属性 -->
<Grid>
    <TextBlock Text="第一行第一列" Grid.Row="0" Grid.Column="0"/>
    <TextBlock Text="第一行第二列" Grid.Row="0" Grid.Column="1"/>
</Grid>
```

### 3.4 x:Name 和 x:Key

```xml
<!-- x:Name: 命名元素，可在 Code-Behind 中访问 -->
<TextBlock x:Name="statusText" Text="就绪"/>

<!-- x:Key: 资源字典中的键 -->
<ResourceDictionary>
    <SolidColorBrush x:Key="AccentBrush" Color="#0078D4"/>
    <Style x:Key="ButtonStyle" TargetType="Button">
        <!-- 样式定义 -->
    </Style>
</ResourceDictionary>
```

### 3.5 标记扩展

**绑定扩展**

```xml
<TextBlock Text="{Binding Path=Name}"/>
<TextBlock Text="{Binding Path=User.Name, Mode=OneWay}"/>
<TextBox Text="{Binding Path=Username, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}"/>
```

**资源扩展**

```xml
<!-- StaticResource: 静态资源，在加载时解析 -->
<Button Background="{StaticResource AccentBrush}"/>

<!-- DynamicResource: 动态资源，运行时可更新 -->
<Button Background="{DynamicResource ThemeBackgroundBrush}"/>
```

**模板绑定**

```xml
<ControlTemplate TargetType="Button">
    <Border Background="{TemplateBinding Background}"
            BorderBrush="{TemplateBinding BorderBrush}"
            BorderThickness="{TemplateBinding BorderThickness}">
        <ContentPresenter/>
    </Border>
</ControlTemplate>
```

**其他扩展**

```xml
<!-- x:Null -->
<Button Command="{x:Null}"/>

<!-- x:Static -->
<TextBlock Text="{x:Static local:Constants.AppName}"/>

<!-- x:Type -->
<DataTemplate DataType="{x:Type local:Person}">
    <!-- 模板内容 -->
</DataTemplate>

<!-- x:Array -->
<x:Array Type="{x:Type x:String}">
    <x:String>项 1</x:String>
    <x:String>项 2</x:String>
</x:Array>
```

### 3.6 附加属性

```xml
<Grid>
    <Grid.RowDefinitions>
        <RowDefinition Height="Auto"/>
        <RowDefinition Height="*"/>
    </Grid.RowDefinitions>
    
    <TextBlock Grid.Row="0" Grid.Column="0" Text="标题"/>
    <TextBlock Grid.Row="1" Grid.Column="0" Text="内容"/>
</Grid>

<!-- DockPanel 附加属性 -->
<DockPanel>
    <Button DockPanel.Dock="Top" Content="顶部"/>
    <Button DockPanel.Dock="Bottom" Content="底部"/>
</DockPanel>

<!-- Canvas 附加属性 -->
<Canvas>
    <Button Canvas.Left="100" Canvas.Top="50" Content="定位按钮"/>
</Canvas>
```

---

## 4. 布局系统

### 4.1 布局基础

Jalium.UI 使用两遍布局系统：
1. **Measure（测量）**：确定每个元素需要的空间
2. **Arrange（排列）**：确定每个元素的最终位置和大小

### 4.2 布局属性

**尺寸属性**

```xml
<UIElement 
    Width="100"           <!-- 固定宽度 -->
    Height="50"           <!-- 固定高度 -->
    MinWidth="50"         <!-- 最小宽度 -->
    MinHeight="25"        <!-- 最小高度 -->
    MaxWidth="200"        <!-- 最大宽度 -->
    MaxHeight="100"       <!-- 最大高度 -->
/>
```

**对齐属性**

```xml
<UIElement
    HorizontalAlignment="Center"  <!-- Left, Center, Right, Stretch -->
    VerticalAlignment="Center"    <!-- Top, Center, Bottom, Stretch -->
/>
```

**边距和填充**

```xml
<!-- Margin: 外边距 -->
<Border Margin="10"/>                    <!-- 四边相同 -->
<Border Margin="10,20"/>                 <!-- 左右, 上下 -->
<Border Margin="10,20,30,40"/>           <!-- 左, 上, 右, 下 -->

<!-- Padding: 内边距 -->
<Border Padding="15"/>
```

### 4.3 Grid（网格布局）

Grid 是最灵活的布局容器，使用行和列定义网格：

```xml
<Grid>
    <Grid.RowDefinitions>
        <RowDefinition Height="Auto"/>      <!-- 自动高度 -->
        <RowDefinition Height="*"/>         <!-- 占用剩余空间 -->
        <RowDefinition Height="2*"/>        <!-- 占用两倍剩余空间 -->
        <RowDefinition Height="100"/>       <!-- 固定高度 -->
    </Grid.RowDefinitions>
    
    <Grid.ColumnDefinitions>
        <ColumnDefinition Width="200"/>     <!-- 固定宽度 -->
        <ColumnDefinition Width="*"/>       <!-- 占用剩余空间 -->
        <ColumnDefinition Width="Auto"/>    <!-- 自动宽度 -->
    </Grid.ColumnDefinitions>
    
    <!-- 标题栏 -->
    <TextBlock Grid.Row="0" Grid.Column="0" Grid.ColumnSpan="3" 
               Text="应用标题" FontSize="24"/>
    
    <!-- 侧边栏 -->
    <Border Grid.Row="1" Grid.Column="0" Background="LightGray">
        <TextBlock Text="侧边栏"/>
    </Border>
    
    <!-- 主内容 -->
    <Border Grid.Row="1" Grid.Column="1" Grid.ColumnSpan="2">
        <TextBlock Text="主内容区域"/>
    </Border>
    
    <!-- 状态栏 -->
    <StatusBar Grid.Row="3" Grid.Column="0" Grid.ColumnSpan="3">
        <TextBlock Text="状态: 就绪"/>
    </StatusBar>
</Grid>
```

**GridLength 值类型**

| 值 | 说明 |
|---|---|
| `Auto` | 根据内容自动调整大小 |
| `*` | 占用剩余空间的等分比例 |
| `2*` | 占用两倍比例的剩余空间 |
| `100` | 固定大小（设备无关单位） |

### 4.4 StackPanel（堆栈布局）

StackPanel 将子元素按水平或垂直方向排列：

```xml
<!-- 垂直堆栈（默认） -->
<StackPanel Orientation="Vertical" Spacing="8">
    <Button Content="按钮 1"/>
    <Button Content="按钮 2"/>
    <Button Content="按钮 3"/>
</StackPanel>

<!-- 水平堆栈 -->
<StackPanel Orientation="Horizontal" Spacing="12">
    <TextBlock Text="用户名:" VerticalAlignment="Center"/>
    <TextBox Width="200"/>
    <Button Content="登录" Margin="8,0,0,0"/>
</StackPanel>
```

### 4.5 DockPanel（停靠布局）

DockPanel 将子元素停靠在容器的边缘：

```xml
<DockPanel LastChildFill="True">
    <Button DockPanel.Dock="Top" Content="顶部工具栏" Height="40"/>
    <Button DockPanel.Dock="Bottom" Content="状态栏" Height="30"/>
    <Button DockPanel.Dock="Left" Content="侧边栏" Width="200"/>
    <Button DockPanel.Dock="Right" Content="属性面板" Width="250"/>
    
    <!-- 中央内容区域（LastChildFill=True 时自动填充） -->
    <Border Background="White">
        <TextBlock Text="主内容区域" HorizontalAlignment="Center" VerticalAlignment="Center"/>
    </Border>
</DockPanel>
```

### 4.6 Canvas（绝对定位）

Canvas 使用坐标绝对定位子元素：

```xml
<Canvas Width="400" Height="300">
    <Rectangle Canvas.Left="50" Canvas.Top="50" 
               Width="100" Height="100" Fill="Red"/>
    <Ellipse Canvas.Left="200" Canvas.Top="100" 
             Width="80" Height="80" Fill="Blue"/>
    <TextBlock Canvas.Left="100" Canvas.Top="220" Text="画布上的文本"/>
</Canvas>
```

### 4.7 WrapPanel（自动换行布局）

WrapPanel 将子元素按顺序排列，空间不足时自动换行：

```xml
<WrapPanel Orientation="Horizontal" ItemWidth="100" ItemHeight="100">
    <Border Background="Red"/>
    <Border Background="Green"/>
    <Border Background="Blue"/>
    <Border Background="Yellow"/>
    <Border Background="Purple"/>
    <Border Background="Orange"/>
</WrapPanel>
```

### 4.8 UniformGrid（均匀网格）

UniformGrid 自动创建均匀大小的网格：

```xml
<UniformGrid Rows="3" Columns="3" FirstColumn="0">
    <Button Content="1"/>
    <Button Content="2"/>
    <Button Content="3"/>
    <Button Content="4"/>
    <Button Content="5"/>
    <Button Content="6"/>
    <Button Content="7"/>
    <Button Content="8"/>
    <Button Content="9"/>
</UniformGrid>
```

### 4.9 Border（边框容器）

Border 为单个子元素添加边框和背景：

```xml
<Border Background="White"
        BorderBrush="Gray"
        BorderThickness="1"
        CornerRadius="8"
        Padding="16">
    <TextBlock Text="带边框的内容"/>
</Border>
```

---

## 5. 控件库

### 5.1 基础控件

**Button（按钮）**

```xml
<Button Content="点击我"
        Width="100"
        Height="32"
        Click="OnButtonClick"
        IsDefault="True"
        IsCancel="False"/>
```

```csharp
// 代码创建
var button = new Button
{
    Content = "点击我",
    Width = 100,
    Height = 32,
    IsDefault = true
};
button.Click += (s, e) => { /* 处理点击 */ };
```

**TextBox（文本框）**

```xml
<TextBox Text="{Binding Username, Mode=TwoWay}"
         Width="200"
         Height="30"
         MaxLength="100"
         IsReadOnly="False"
         AcceptsReturn="True"
         TextWrapping="Wrap"/>
```

**TextBlock（文本块）**

```xml
<TextBlock Text="显示文本"
           FontSize="16"
           FontWeight="Bold"
           Foreground="Blue"
           TextWrapping="Wrap"
           TextTrimming="CharacterEllipsis"/>
```

**CheckBox（复选框）**

```xml
<CheckBox Content="记住我"
          IsChecked="{Binding RememberMe, Mode=TwoWay}"
          IsThreeState="False"/>
```

**RadioButton（单选按钮）**

```xml
<StackPanel>
    <RadioButton Content="选项 A" GroupName="Options" IsChecked="True"/>
    <RadioButton Content="选项 B" GroupName="Options"/>
    <RadioButton Content="选项 C" GroupName="Options"/>
</StackPanel>
```

**ComboBox（下拉框）**

```xml
<ComboBox Width="200"
          SelectedIndex="0"
          SelectedItem="{Binding SelectedItem}">
    <ComboBoxItem Content="项目 1"/>
    <ComboBoxItem Content="项目 2"/>
    <ComboBoxItem Content="项目 3"/>
</ComboBox>
```

**ListBox（列表框）**

```xml
<ListBox ItemsSource="{Binding Items}"
         SelectedItem="{Binding SelectedItem}"
         DisplayMemberPath="Name">
</ListBox>
```

### 5.2 范围控件

**Slider（滑块）**

```xml
<Slider Minimum="0"
        Maximum="100"
        Value="{Binding Volume, Mode=TwoWay}"
        TickFrequency="10"
        IsSnapToTickEnabled="True"/>
```

**ProgressBar（进度条）**

```xml
<ProgressBar Minimum="0"
             Maximum="100"
             Value="50"
             Width="200"
             Height="20"/>
```

**ScrollBar（滚动条）**

```xml
<ScrollBar Orientation="Horizontal"
           Minimum="0"
           Maximum="100"
           ViewportSize="10"
           Value="50"/>
```

### 5.3 日期时间控件

**Calendar（日历）**

```xml
<Calendar SelectedDate="{Binding SelectedDate, Mode=TwoWay}"
          DisplayDateStart="2024-01-01"
          DisplayDateEnd="2024-12-31"
          SelectionMode="SingleDate"/>
```

**DatePicker（日期选择器）**

```xml
<DatePicker SelectedDate="{Binding BirthDate, Mode=TwoWay}"
            Width="200"
            Format="yyyy-MM-dd"/>
```

### 5.4 菜单和工具栏

**Menu（菜单）**

```xml
<Menu>
    <MenuItem Header="文件">
        <MenuItem Header="新建" Command="{Binding NewCommand}" InputGestureText="Ctrl+N"/>
        <MenuItem Header="打开" Command="{Binding OpenCommand}"/>
        <Separator/>
        <MenuItem Header="退出" Command="{Binding ExitCommand}"/>
    </MenuItem>
    <MenuItem Header="编辑">
        <MenuItem Header="复制" Command="{Binding CopyCommand}"/>
        <MenuItem Header="粘贴" Command="{Binding PasteCommand}"/>
    </MenuItem>
</Menu>
```

**ContextMenu（上下文菜单）**

```xml
<TextBox Text="右键点击我">
    <TextBox.ContextMenu>
        <ContextMenu>
            <MenuItem Header="复制" Command="Copy"/>
            <MenuItem Header="粘贴" Command="Paste"/>
            <Separator/>
            <MenuItem Header="全选" Command="SelectAll"/>
        </ContextMenu>
    </TextBox.ContextMenu>
</TextBox>
```

**ToolBar（工具栏）**

```xml
<ToolBarTray>
    <ToolBar Band="0" BandIndex="0">
        <Button Content="新建" Command="{Binding NewCommand}"/>
        <Button Content="打开" Command="{Binding OpenCommand}"/>
        <Separator/>
        <Button Content="保存" Command="{Binding SaveCommand}"/>
    </ToolBar>
    <ToolBar Band="0" BandIndex="1">
        <Button Content="撤销" Command="{Binding UndoCommand}"/>
        <Button Content="重做" Command="{Binding RedoCommand}"/>
    </ToolBar>
</ToolBarTray>
```

### 5.5 导航控件

**TabControl（选项卡）**

```xml
<TabControl>
    <TabItem Header="常规">
        <TextBlock Text="常规设置内容"/>
    </TabItem>
    <TabItem Header="高级">
        <TextBlock Text="高级设置内容"/>
    </TabItem>
    <TabItem Header="关于">
        <TextBlock Text="关于本程序"/>
    </TabItem>
</TabControl>
```

**TreeView（树视图）**

```xml
<TreeView ItemsSource="{Binding RootNodes}">
    <TreeView.ItemTemplate>
        <HierarchicalDataTemplate ItemsSource="{Binding Children}">
            <TextBlock Text="{Binding Name}"/>
        </HierarchicalDataTemplate>
    </TreeView.ItemTemplate>
</TreeView>
```

**NavigationView（导航视图）**

```xml
<NavigationView>
    <NavigationViewItem Icon="Home" Content="首页"/>
    <NavigationViewItem Icon="Settings" Content="设置"/>
    <NavigationViewItem Icon="Help" Content="帮助"/>
</NavigationView>
```

### 5.6 数据控件

**DataGrid（数据表格）**

```xml
<DataGrid ItemsSource="{Binding Users}"
          AutoGenerateColumns="False"
          IsReadOnly="True"
          SelectedItem="{Binding SelectedUser}">
    <DataGrid.Columns>
        <DataGridTextColumn Header="姓名" Binding="{Binding Name}" Width="*"/>
        <DataGridTextColumn Header="邮箱" Binding="{Binding Email}" Width="*"/>
        <DataGridTemplateColumn Header="操作" Width="Auto">
            <DataGridTemplateColumn.CellTemplate>
                <DataTemplate>
                    <StackPanel Orientation="Horizontal">
                        <Button Content="编辑" Command="{Binding EditCommand}"/>
                        <Button Content="删除" Command="{Binding DeleteCommand}" Margin="4,0,0,0"/>
                    </StackPanel>
                </DataTemplate>
            </DataGridTemplateColumn.CellTemplate>
        </DataGridTemplateColumn>
    </DataGrid.Columns>
</DataGrid>
```

**ListView（列表视图）**

```xml
<ListView ItemsSource="{Binding Items}">
    <ListView.View>
        <GridView>
            <GridViewColumn Header="名称" DisplayMemberBinding="{Binding Name}"/>
            <GridViewColumn Header="大小" DisplayMemberBinding="{Binding Size}"/>
            <GridViewColumn Header="日期" DisplayMemberBinding="{Binding Date}"/>
        </GridView>
    </ListView.View>
</ListView>
```

### 5.7 对话框控件

**MessageBox**

```csharp
// 简单消息
MessageBox.Show("操作成功！");

// 带标题和按钮
var result = MessageBox.Show(
    "确定要删除吗？",
    "确认删除",
    MessageBoxButton.YesNo,
    MessageBoxImage.Warning
);

if (result == MessageBoxResult.Yes)
{
    // 执行删除操作
}
```

**FileDialog**

```csharp
// 打开文件对话框
var openDialog = new OpenFileDialog
{
    Title = "选择文件",
    Filter = "文本文件|*.txt|所有文件|*.*",
    Multiselect = false
};

if (openDialog.ShowDialog() == true)
{
    var filePath = openDialog.FileName;
    // 处理文件
}

// 保存文件对话框
var saveDialog = new SaveFileDialog
{
    Title = "保存文件",
    Filter = "文本文件|*.txt",
    DefaultExt = ".txt"
};

if (saveDialog.ShowDialog() == true)
{
    var filePath = saveDialog.FileName;
    // 保存文件
}
```

### 5.8 容器控件

**Expander（折叠面板）**

```xml
<Expander Header="高级选项" IsExpanded="False">
    <StackPanel Margin="8">
        <CheckBox Content="启用调试模式"/>
        <CheckBox Content="显示详细日志"/>
        <CheckBox Content="自动保存"/>
    </StackPanel>
</Expander>
```

**GroupBox（分组框）**

```xml
<GroupBox Header="个人信息">
    <StackPanel Margin="8">
        <TextBox Watermark="请输入姓名" Margin="0,0,0,8"/>
        <TextBox Watermark="请输入邮箱"/>
    </StackPanel>
</GroupBox>
```

**ScrollViewer（滚动容器）**

```xml
<ScrollViewer HorizontalScrollBarVisibility="Auto"
              VerticalScrollBarVisibility="Auto">
    <StackPanel>
        <!-- 大量内容 -->
    </StackPanel>
</ScrollViewer>
```

### 5.9 媒体控件

**Image（图像）**

```xml
<Image Source="Images/logo.png"
       Width="200"
       Height="100"
       Stretch="Uniform"
       StretchDirection="Both"/>
```

**MediaElement（媒体元素）**

```xml
<MediaElement Source="Videos/demo.mp4"
              Width="640"
              Height="480"
              AutoPlay="True"
              IsLooping="False"/>
```

---

## 6. 数据绑定

### 6.1 绑定概念

数据绑定是在 UI 控件（绑定目标）和数据对象（绑定源）之间建立连接的机制。

**绑定方向（BindingMode）**

| 模式 | 说明 |
|---|---|
| `OneWay` | 源 → 目标（源变化时更新目标） |
| `TwoWay` | 源 ↔ 目标（双向同步） |
| `OneTime` | 仅初始化时从源读取一次 |
| `OneWayToSource` | 目标 → 源 |
| `Default` | 使用目标属性的默认模式 |

**更新触发器（UpdateSourceTrigger）**

| 触发器 | 说明 |
|---|---|
| `PropertyChanged` | 属性变化时立即更新 |
| `LostFocus` | 失去焦点时更新 |
| `Explicit` | 仅在调用 `UpdateSource()` 时更新 |
| `Default` | 使用目标属性的默认触发器 |

### 6.2 基本绑定

**简单绑定**

```xml
<!-- 绑定到 DataContext 的属性 -->
<TextBlock Text="{Binding Name}"/>

<!-- 完整语法 -->
<TextBlock Text="{Binding Path=Name, Mode=OneWay}"/>
```

**双向绑定**

```xml
<TextBox Text="{Binding Username, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}"/>
```

**绑定到集合**

```xml
<ListBox ItemsSource="{Binding Users}"
         SelectedItem="{Binding SelectedUser}"
         DisplayMemberPath="Name"/>
```

### 6.3 绑定源

**Source 属性 - 直接指定源**

```xml
<TextBlock Text="{Binding Source={StaticResource myDataSource}, Path=Name}"/>
```

**ElementName - 绑定到其他元素**

```xml
<Slider x:Name="volumeSlider" Minimum="0" Maximum="100" Value="50"/>
<TextBlock Text="{Binding ElementName=volumeSlider, Path=Value, StringFormat='音量: {0}%'}"/>
```

**RelativeSource - 相对绑定**

```xml
<!-- 绑定到自身 -->
<TextBlock Text="{Binding RelativeSource={RelativeSource Self}, Path=ActualWidth, StringFormat='宽度: {0}'}"/>

<!-- 绑定到模板父级 -->
<ControlTemplate TargetType="Button">
    <Border Background="{Binding RelativeSource={RelativeSource TemplatedParent}, Path=Background}"/>
</ControlTemplate>

<!-- 绑定到祖先元素 -->
<TextBlock Text="{Binding RelativeSource={RelativeSource FindAncestor, AncestorType={x:Type Window}}, Path=Title}"/>
```

**DataContext - 数据上下文**

```csharp
// 设置数据上下文
public class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        DataContext = new MainViewModel();
    }
}
```

```xml
<!-- 子元素继承 DataContext -->
<StackPanel DataContext="{Binding User}">
    <TextBlock Text="{Binding Name}"/>
    <TextBlock Text="{Binding Email}"/>
</StackPanel>
```

### 6.4 值转换器

**创建值转换器**

```csharp
public class BoolToVisibilityConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        return (value is bool b && b) ? Visibility.Visible : Visibility.Collapsed;
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        return (value is Visibility v && v == Visibility.Visible);
    }
}
```

**使用值转换器**

```xml
<Window.Resources>
    <local:BoolToVisibilityConverter x:Key="BoolToVisibilityConverter"/>
</Window.Resources>

<Border Visibility="{Binding IsVisible, Converter={StaticResource BoolToVisibilityConverter}}">
    <TextBlock Text="条件显示的内容"/>
</Border>
```

**带参数的转换器**

```xml
<TextBlock Text="{Binding Price, Converter={StaticResource CurrencyConverter}, ConverterParameter='C'}"/>
```

### 6.5 多值绑定

**IMultiValueConverter**

```csharp
public class StringFormatConverter : IMultiValueConverter
{
    public object? Convert(object?[] values, Type targetType, object? parameter, CultureInfo culture)
    {
        if (values == null || values.Length < 2)
            return null;
        
        var firstName = values[0]?.ToString() ?? "";
        var lastName = values[1]?.ToString() ?? "";
        return $"{firstName} {lastName}";
    }

    public object?[] ConvertBack(object? value, Type[] targetTypes, object? parameter, CultureInfo culture)
    {
        throw new NotImplementedException();
    }
}
```

**MultiBinding 使用**

```xml
<TextBlock>
    <TextBlock.Text>
        <MultiBinding Converter="{StaticResource StringFormatConverter}">
            <Binding Path="FirstName"/>
            <Binding Path="LastName"/>
        </MultiBinding>
    </TextBlock.Text>
</TextBlock>
```

### 6.6 绑定验证

**ValidationRule**

```csharp
public class AgeValidationRule : ValidationRule
{
    public int MinAge { get; set; } = 0;
    public int MaxAge { get; set; } = 120;

    public override ValidationResult Validate(object? value, CultureInfo cultureInfo)
    {
        if (value is not int age)
        {
            return new ValidationResult(false, "请输入有效的年龄");
        }

        if (age < MinAge || age > MaxAge)
        {
            return new ValidationResult(false, $"年龄必须在 {MinAge} 到 {MaxAge} 之间");
        }

        return ValidationResult.ValidResult;
    }
}
```

**在绑定中使用验证**

```xml
<TextBox>
    <TextBox.Text>
        <Binding Path="Age" UpdateSourceTrigger="PropertyChanged">
            <Binding.ValidationRules>
                <local:AgeValidationRule MinAge="0" MaxAge="120"/>
            </Binding.ValidationRules>
        </Binding>
    </TextBox.Text>
</TextBox>
```

**IDataErrorInfo**

```csharp
public class Person : IDataErrorInfo
{
    public string Name { get; set; }
    
    public string Error => null;
    
    public string this[string propertyName]
    {
        get
        {
            if (propertyName == nameof(Name))
            {
                if (string.IsNullOrWhiteSpace(Name))
                    return "姓名不能为空";
                if (Name.Length > 50)
                    return "姓名不能超过50个字符";
            }
            return null;
        }
    }
}
```

```xml
<TextBox Text="{Binding Name, ValidatesOnDataErrors=True}"/>
```

### 6.7 StringFormat

```xml
<!-- 数字格式化 -->
<TextBlock Text="{Binding Price, StringFormat='{}{0:C}'}"/>           <!-- 货币格式 -->
<TextBlock Text="{Binding Count, StringFormat='数量: {0:N0}'}"/>      <!-- 整数格式 -->
<TextBlock Text="{Binding Rate, StringFormat='{}{0:P}'}"/>            <!-- 百分比格式 -->

<!-- 日期格式化 -->
<TextBlock Text="{Binding Date, StringFormat='{}{0:yyyy-MM-dd}'}"/>
<TextBlock Text="{Binding Date, StringFormat='{}{0:yyyy年MM月dd日}'}"/>

<!-- 自定义格式 -->
<TextBlock Text="{Binding Name, StringFormat='姓名: {0}'}"/>
```

### 6.8 FallbackValue 和 TargetNullValue

```xml
<!-- FallbackValue: 绑定失败时显示的值 -->
<TextBlock Text="{Binding Name, FallbackValue='(未知)'}"/>

<!-- TargetNullValue: 源值为 null 时显示的值 -->
<TextBlock Text="{Binding NickName, TargetNullValue='(无昵称)'}"/>
```

---

## 7. 资源和样式

### 7.1 资源字典

**定义资源**

```xml
<Window.Resources>
    <!-- 颜色资源 -->
    <Color x:Key="AccentColor">#0078D4</Color>
    
    <!-- 画刷资源 -->
    <SolidColorBrush x:Key="AccentBrush" Color="{StaticResource AccentColor}"/>
    
    <!-- 几何资源 -->
    <Thickness x:Key="DefaultMargin">8</Thickness>
    <Thickness x:Key="ControlPadding">12,6</Thickness>
    
    <!-- 字体资源 -->
    <FontFamily x:Key="DefaultFont">Segoe UI</FontFamily>
</Window.Resources>
```

**使用资源**

```xml
<Button Background="{StaticResource AccentBrush}"
        Padding="{StaticResource ControlPadding}"
        FontFamily="{StaticResource DefaultFont}"
        Content="按钮"/>
```

### 7.2 资源查找顺序

资源按以下顺序查找：
1. 元素自身的 Resources
2. 父元素的 Resources
3. 应用程序的 Resources
4. 主题资源字典

```xml
<Window>
    <Window.Resources>
        <SolidColorBrush x:Key="MyBrush" Color="Red"/>
    </Window.Resources>
    
    <StackPanel>
        <StackPanel.Resources>
            <SolidColorBrush x:Key="MyBrush" Color="Blue"/>
        </StackPanel.Resources>
        
        <!-- 使用蓝色画刷（就近原则） -->
        <Button Background="{StaticResource MyBrush}" Content="按钮"/>
    </StackPanel>
</Window>
```

### 7.3 合并资源字典

```xml
<Window.Resources>
    <ResourceDictionary>
        <ResourceDictionary.MergedDictionaries>
            <ResourceDictionary Source="Themes/Colors.jalxaml"/>
            <ResourceDictionary Source="Themes/Brushes.jalxaml"/>
            <ResourceDictionary Source="Themes/Styles.jalxaml"/>
        </ResourceDictionary.MergedDictionaries>
        
        <!-- 当前窗口的本地资源 -->
        <SolidColorBrush x:Key="LocalBrush" Color="Green"/>
    </ResourceDictionary>
</Window.Resources>
```

### 7.4 主题字典

```xml
<ResourceDictionary>
    <ResourceDictionary.ThemeDictionaries>
        <ResourceDictionary x:Key="Light">
            <SolidColorBrush x:Key="BackgroundBrush" Color="White"/>
            <SolidColorBrush x:Key="ForegroundBrush" Color="Black"/>
        </ResourceDictionary>
        
        <ResourceDictionary x:Key="Dark">
            <SolidColorBrush x:Key="BackgroundBrush" Color="#1E1E1E"/>
            <SolidColorBrush x:Key="ForegroundBrush" Color="White"/>
        </ResourceDictionary>
    </ResourceDictionary.ThemeDictionaries>
</ResourceDictionary>
```

### 7.5 样式基础

**隐式样式**

```xml
<!-- 隐式样式：应用于所有 Button -->
<Style TargetType="Button">
    <Setter Property="Background" Value="LightGray"/>
    <Setter Property="Foreground" Value="Black"/>
    <Setter Property="Padding" Value="12,6"/>
    <Setter Property="FontSize" Value="14"/>
</Style>

<!-- 所有 Button 自动应用此样式 -->
<Button Content="自动样式"/>
```

**显式样式**

```xml
<!-- 显式样式：需要通过 Key 引用 -->
<Style x:Key="AccentButtonStyle" TargetType="Button">
    <Setter Property="Background" Value="{StaticResource AccentBrush}"/>
    <Setter Property="Foreground" Value="White"/>
    <Setter Property="Padding" Value="16,8"/>
    <Setter Property="CornerRadius" Value="4"/>
</Style>

<Button Style="{StaticResource AccentButtonStyle}" Content="强调按钮"/>
```

### 7.6 样式继承

```xml
<!-- 基础样式 -->
<Style x:Key="BaseButtonStyle" TargetType="Button">
    <Setter Property="Padding" Value="12,6"/>
    <Setter Property="FontSize" Value="14"/>
    <Setter Property="CornerRadius" Value="4"/>
</Style>

<!-- 继承样式 -->
<Style x:Key="PrimaryButtonStyle" TargetType="Button" BasedOn="{StaticResource BaseButtonStyle}">
    <Setter Property="Background" Value="Blue"/>
    <Setter Property="Foreground" Value="White"/>
</Style>

<Style x:Key="DangerButtonStyle" TargetType="Button" BasedOn="{StaticResource BaseButtonStyle}">
    <Setter Property="Background" Value="Red"/>
    <Setter Property="Foreground" Value="White"/>
</Style>
```

### 7.7 触发器

**属性触发器（PropertyTrigger）**

```xml
<Style TargetType="Button">
    <Setter Property="Background" Value="LightGray"/>
    <Setter Property="Foreground" Value="Black"/>
    
    <Style.Triggers>
        <!-- 鼠标悬停 -->
        <Trigger Property="IsMouseOver" Value="True">
            <Setter Property="Background" Value="Gray"/>
            <Setter Property="Foreground" Value="White"/>
        </Trigger>
        
        <!-- 按下状态 -->
        <Trigger Property="IsPressed" Value="True">
            <Setter Property="Background" Value="DarkGray"/>
        </Trigger>
        
        <!-- 禁用状态 -->
        <Trigger Property="IsEnabled" Value="False">
            <Setter Property="Background" Value="#E0E0E0"/>
            <Setter Property="Foreground" Value="#A0A0A0"/>
        </Trigger>
    </Style.Triggers>
</Style>
```

**多条件触发器（MultiTrigger）**

```xml
<Style TargetType="TextBox">
    <Style.Triggers>
        <MultiTrigger>
            <MultiTrigger.Conditions>
                <Condition Property="IsMouseOver" Value="True"/>
                <Condition Property="IsFocused" Value="True"/>
            </MultiTrigger.Conditions>
            <Setter Property="BorderBrush" Value="Blue"/>
        </MultiTrigger>
    </Style.Triggers>
</Style>
```

**数据触发器（DataTrigger）**

```xml
<Style TargetType="TextBlock">
    <Setter Property="Foreground" Value="Black"/>
    
    <Style.Triggers>
        <DataTrigger Binding="{Binding Status}" Value="Error">
            <Setter Property="Foreground" Value="Red"/>
        </DataTrigger>
        
        <DataTrigger Binding="{Binding Status}" Value="Warning">
            <Setter Property="Foreground" Value="Orange"/>
        </DataTrigger>
        
        <DataTrigger Binding="{Binding Status}" Value="Success">
            <Setter Property="Foreground" Value="Green"/>
        </DataTrigger>
    </Style.Triggers>
</Style>
```

**事件触发器（EventTrigger）**

```xml
<Style TargetType="Button">
    <Style.Triggers>
        <EventTrigger RoutedEvent="MouseEnter">
            <BeginStoryboard>
                <Storyboard>
                    <ColorAnimation Storyboard.TargetProperty="(Button.Background).(SolidColorBrush.Color)"
                                   To="LightBlue" Duration="0:0:0.2"/>
                </Storyboard>
            </BeginStoryboard>
        </EventTrigger>
        
        <EventTrigger RoutedEvent="MouseLeave">
            <BeginStoryboard>
                <Storyboard>
                    <ColorAnimation Storyboard.TargetProperty="(Button.Background).(SolidColorBrush.Color)"
                                   To="White" Duration="0:0:0.2"/>
                </Storyboard>
            </BeginStoryboard>
        </EventTrigger>
    </Style.Triggers>
</Style>
```

---

## 8. 控件模板

### 8.1 模板基础

控件模板定义控件的可视化结构：

```xml
<ControlTemplate x:Key="SimpleButtonTemplate" TargetType="Button">
    <Border Background="{TemplateBinding Background}"
            BorderBrush="{TemplateBinding BorderBrush}"
            BorderThickness="{TemplateBinding BorderThickness}"
            CornerRadius="4"
            Padding="{TemplateBinding Padding}">
        <ContentPresenter HorizontalAlignment="Center"
                         VerticalAlignment="Center"/>
    </Border>
</ControlTemplate>

<Button Template="{StaticResource SimpleButtonTemplate}"
        Content="自定义按钮"
        Background="LightBlue"/>
```

### 8.2 TemplateBinding

`TemplateBinding` 用于将模板中的属性绑定到模板父控件的属性：

```xml
<ControlTemplate TargetType="Button">
    <Border x:Name="Root"
            Background="{TemplateBinding Background}"
            BorderBrush="{TemplateBinding BorderBrush}"
            BorderThickness="{TemplateBinding BorderThickness}"
            CornerRadius="{TemplateBinding CornerRadius}">
        
        <!-- ContentPresenter 自动绑定到 Content 属性 -->
        <ContentPresenter HorizontalAlignment="{TemplateBinding HorizontalContentAlignment}"
                         VerticalAlignment="{TemplateBinding VerticalContentAlignment}"
                         Margin="{TemplateBinding Padding}"/>
    </Border>
</ControlTemplate>
```

### 8.3 模板中的触发器

```xml
<ControlTemplate TargetType="Button">
    <Border x:Name="RootBorder"
            Background="{TemplateBinding Background}"
            BorderBrush="{TemplateBinding BorderBrush}"
            BorderThickness="1"
            CornerRadius="4">
        <ContentPresenter HorizontalAlignment="Center"
                         VerticalAlignment="Center"/>
    </Border>
    
    <ControlTemplate.Triggers>
        <Trigger Property="IsMouseOver" Value="True">
            <Setter TargetName="RootBorder" Property="Background" Value="LightGray"/>
        </Trigger>
        
        <Trigger Property="IsPressed" Value="True">
            <Setter TargetName="RootBorder" Property="Background" Value="Gray"/>
            <Setter TargetName="RootBorder" Property="RenderTransform">
                <Setter.Value>
                    <ScaleTransform ScaleX="0.98" ScaleY="0.98"/>
                </Setter.Value>
            </Setter>
        </Trigger>
        
        <Trigger Property="IsEnabled" Value="False">
            <Setter TargetName="RootBorder" Property="Opacity" Value="0.5"/>
        </Trigger>
    </ControlTemplate.Triggers>
</ControlTemplate>
```

### 8.4 集成到样式中

```xml
<Style TargetType="Button">
    <Setter Property="Background" Value="White"/>
    <Setter Property="Foreground" Value="Black"/>
    <Setter Property="Padding" Value="12,6"/>
    <Setter Property="Template">
        <Setter.Value>
            <ControlTemplate TargetType="Button">
                <!-- 模板内容 -->
            </ControlTemplate>
        </Setter.Value>
    </Setter>
</Style>
```

### 8.5 数据模板（DataTemplate）

数据模板定义数据对象的可视化表示：

```xml
<!-- 定义数据模板 -->
<DataTemplate x:Key="PersonTemplate" DataType="{x:Type local:Person}">
    <Border Background="White" Padding="8" CornerRadius="4">
        <StackPanel>
            <TextBlock Text="{Binding Name}" FontWeight="Bold" FontSize="16"/>
            <TextBlock Text="{Binding Email}" Foreground="Gray"/>
            <TextBlock Text="{Binding Phone}" Foreground="Gray"/>
        </StackPanel>
    </Border>
</DataTemplate>

<!-- 使用数据模板 -->
<ContentControl Content="{Binding SelectedPerson}"
               ContentTemplate="{StaticResource PersonTemplate}"/>
```

**隐式数据模板**

```xml
<!-- 根据 DataType 自动应用 -->
<DataTemplate DataType="{x:Type local:Person}">
    <TextBlock Text="{Binding Name}"/>
</DataTemplate>

<DataTemplate DataType="{x:Type local:Company}">
    <TextBlock Text="{Binding CompanyName}" FontWeight="Bold"/>
</DataTemplate>

<!-- 自动选择模板 -->
<ContentControl Content="{Binding CurrentItem}"/>
```

**HierarchicalDataTemplate**

用于树形结构数据：

```xml
<HierarchicalDataTemplate DataType="{x:Type local:Node}" ItemsSource="{Binding Children}">
    <StackPanel Orientation="Horizontal">
        <Image Source="folder.png" Width="16" Height="16" Margin="0,0,4,0"/>
        <TextBlock Text="{Binding Name}"/>
    </StackPanel>
</HierarchicalDataTemplate>
```

### 8.6 ItemsPanelTemplate

定义 ItemsControl 的面板布局：

```xml
<ItemsControl ItemsSource="{Binding Items}">
    <ItemsControl.ItemsPanel>
        <ItemsPanelTemplate>
            <WrapPanel Orientation="Horizontal"/>
        </ItemsPanelTemplate>
    </ItemsControl.ItemsPanel>
</ItemsControl>
```

---

## 9. 可视化状态管理

### 9.1 概念

可视化状态（Visual State）定义控件在不同状态下的外观。状态组织在状态组（VisualStateGroup）中，同一组内的状态互斥。

### 9.2 定义可视化状态

```xml
<ControlTemplate TargetType="Button">
    <Grid>
        <VisualStateManager.VisualStateGroups>
            <VisualStateGroup x:Name="CommonStates">
                <!-- 正常状态 -->
                <VisualState x:Name="Normal"/>
                
                <!-- 鼠标悬停状态 -->
                <VisualState x:Name="MouseOver">
                    <Storyboard>
                        <ColorAnimation Storyboard.TargetName="RootBorder"
                                       Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                       To="#E0E0E0" Duration="0:0:0.15"/>
                    </Storyboard>
                </VisualState>
                
                <!-- 按下状态 -->
                <VisualState x:Name="Pressed">
                    <Storyboard>
                        <ColorAnimation Storyboard.TargetName="RootBorder"
                                       Storyboard.TargetProperty="(Border.Background).(SolidColorBrush.Color)"
                                       To="#C0C0C0" Duration="0"/>
                    </Storyboard>
                </VisualState>
                
                <!-- 禁用状态 -->
                <VisualState x:Name="Disabled">
                    <Storyboard>
                        <DoubleAnimation Storyboard.TargetName="RootBorder"
                                        Storyboard.TargetProperty="Opacity"
                                        To="0.5" Duration="0"/>
                    </Storyboard>
                </VisualState>
            </VisualStateGroup>
            
            <VisualStateGroup x:Name="FocusStates">
                <VisualState x:Name="Focused">
                    <Storyboard>
                        <ColorAnimation Storyboard.TargetName="FocusBorder"
                                       Storyboard.TargetProperty="(Border.BorderBrush).(SolidColorBrush.Color)"
                                       To="Blue" Duration="0"/>
                    </Storyboard>
                </VisualState>
                
                <VisualState x:Name="Unfocused"/>
            </VisualStateGroup>
        </VisualStateManager.VisualStateGroups>
        
        <Border x:Name="RootBorder" Background="White" CornerRadius="4">
            <ContentPresenter HorizontalAlignment="Center" VerticalAlignment="Center"/>
        </Border>
        <Border x:Name="FocusBorder" BorderBrush="Transparent" BorderThickness="2" CornerRadius="4"/>
    </Grid>
</ControlTemplate>
```

### 9.3 状态过渡

```xml
<VisualStateGroup x:Name="CommonStates">
    <VisualStateGroup.Transitions>
        <!-- 从任意状态到 MouseOver 的过渡 -->
        <VisualTransition From="*" To="MouseOver" GeneratedDuration="0:0:0.2">
            <VisualTransition.GeneratedEasingFunction>
                <CubicEase EasingMode="EaseOut"/>
            </VisualTransition.GeneratedEasingFunction>
        </VisualTransition>
        
        <!-- 从 MouseOver 到 Normal 的过渡 -->
        <VisualTransition From="MouseOver" To="Normal" GeneratedDuration="0:0:0.3"/>
    </VisualStateGroup.Transitions>
    
    <VisualState x:Name="Normal"/>
    <VisualState x:Name="MouseOver">...</VisualState>
</VisualStateGroup>
```

### 9.4 代码中切换状态

```csharp
public class MyButton : Button
{
    protected override void OnMouseEnter(MouseEventArgs e)
    {
        base.OnMouseEnter(e);
        VisualStateManager.GoToState(this, "MouseOver", useTransitions: true);
    }
    
    protected override void OnMouseLeave(MouseEventArgs e)
    {
        base.OnMouseLeave(e);
        VisualStateManager.GoToState(this, "Normal", useTransitions: true);
    }
    
    protected override void OnIsEnabledChanged(DependencyPropertyChangedEventArgs e)
    {
        base.OnIsEnabledChanged(e);
        var state = (bool)e.NewValue ? "Normal" : "Disabled";
        VisualStateManager.GoToState(this, state, useTransitions: false);
    }
}
```

---

## 10. 路由事件

### 10.1 概念

路由事件是一种可以沿着视觉树传递的事件，支持三种路由策略：

| 策略 | 说明 |
|---|---|
| `Direct` | 仅在源元素上触发 |
| `Bubble` | 从源元素向上冒泡到根元素 |
| `Tunnel` | 从根元素向下隧道到源元素 |

### 10.2 注册路由事件

```csharp
public class MyButton : Button
{
    // 注册路由事件
    public static readonly RoutedEvent MyClickEvent =
        EventManager.RegisterRoutedEvent(
            "MyClick",                        // 事件名称
            RoutingStrategy.Bubble,           // 路由策略
            typeof(RoutedEventHandler),       // 处理器类型
            typeof(MyButton)                  // 拥有者类型
        );
    
    // CLR 事件包装
    public event RoutedEventHandler MyClick
    {
        add => AddHandler(MyClickEvent, value);
        remove => RemoveHandler(MyClickEvent, value);
    }
    
    // 触发事件
    protected virtual void OnMyClick()
    {
        RaiseEvent(new RoutedEventArgs(MyClickEvent, this));
    }
}
```

### 10.3 处理路由事件

**XAML 中处理**

```xml
<Grid Button.Click="OnButtonClick">
    <StackPanel>
        <Button Content="按钮 1"/>
        <Button Content="按钮 2"/>
    </StackPanel>
</Grid>
```

**代码中处理**

```csharp
// 添加处理器
myButton.AddHandler(Button.ClickEvent, new RoutedEventHandler(OnButtonClick));

// 移除处理器
myButton.RemoveHandler(Button.ClickEvent, new RoutedEventHandler(OnButtonClick));

// 类处理器（静态，对所有实例生效）
static MyControl()
{
    EventManager.RegisterClassHandler(
        typeof(MyControl),
        Button.ClickEvent,
        new RoutedEventHandler(OnButtonClickClassHandler)
    );
}
```

### 10.4 标记事件已处理

```csharp
private void OnButtonClick(object sender, RoutedEventArgs e)
{
    // 标记事件已处理，停止路由
    e.Handled = true;
}

// 即使已处理仍然接收事件
myButton.AddHandler(Button.ClickEvent, new RoutedEventHandler(OnButtonClick), handledEventsToo: true);
```

### 10.5 内置路由事件

```xml
<!-- 常用路由事件 -->
<Grid 
    Button.Click="OnButtonClick"
    UIElement.MouseLeftButtonDown="OnMouseLeftButtonDown"
    UIElement.MouseMove="OnMouseMove"
    UIElement.KeyDown="OnKeyDown"
    UIElement.GotFocus="OnGotFocus">
    <!-- 内容 -->
</Grid>
```

---

## 11. 依赖属性

### 11.1 概念

依赖属性是一种支持数据绑定、样式、动画和继承的属性系统。

### 11.2 定义依赖属性

```csharp
public class MyControl : Control
{
    // 1. 注册依赖属性
    public static readonly DependencyProperty ValueProperty =
        DependencyProperty.Register(
            nameof(Value),                    // 属性名称
            typeof(double),                   // 属性类型
            typeof(MyControl),                // 拥有者类型
            new PropertyMetadata(             // 元数据
                0.0,                          // 默认值
                OnValueChanged,               // 属性变更回调
                CoerceValue                   // 强制值回调（可选）
            ),
            ValidateValue                     // 验证回调（可选）
        );
    
    // 2. CLR 包装属性
    public double Value
    {
        get => (double)GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }
    
    // 3. 属性变更回调（静态）
    private static void OnValueChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var control = (MyControl)d;
        var oldValue = (double)e.OldValue;
        var newValue = (double)e.NewValue;
        
        // 响应属性变更
        control.OnValueChanged(oldValue, newValue);
    }
    
    // 4. 实例方法处理变更
    protected virtual void OnValueChanged(double oldValue, double newValue)
    {
        InvalidateVisual();
    }
    
    // 5. 强制值回调
    private static object CoerceValue(DependencyObject d, object baseValue)
    {
        var value = (double)baseValue;
        // 确保值在有效范围内
        return Math.Max(0, Math.Min(100, value));
    }
    
    // 6. 验证回调
    private static bool ValidateValue(object value)
    {
        var d = (double)value;
        return !double.IsNaN(d) && !double.IsInfinity(d);
    }
}
```

### 11.3 只读依赖属性

```csharp
public class MyControl : Control
{
    // 注册只读依赖属性
    private static readonly DependencyPropertyKey IsReadOnlyPropertyKey =
        DependencyProperty.RegisterReadOnly(
            nameof(IsReadOnly),
            typeof(bool),
            typeof(MyControl),
            new PropertyMetadata(false)
        );
    
    public static readonly DependencyProperty IsReadOnlyProperty =
        IsReadOnlyPropertyKey.DependencyProperty;
    
    // 只读 CLR 属性
    public bool IsReadOnly
    {
        get => (bool)GetValue(IsReadOnlyProperty);
        private set => SetValue(IsReadOnlyPropertyKey, value);
    }
}
```

### 11.4 附加属性

```csharp
public class Grid : Panel
{
    // 注册附加属性
    public static readonly DependencyProperty RowProperty =
        DependencyProperty.RegisterAttached(
            "Row",                            // 属性名称
            typeof(int),                      // 属性类型
            typeof(Grid),                     // 拥有者类型
            new PropertyMetadata(0)           // 默认值
        );
    
    // Getter
    public static int GetRow(UIElement element)
    {
        return (int)element.GetValue(RowProperty);
    }
    
    // Setter
    public static void SetRow(UIElement element, int value)
    {
        element.SetValue(RowProperty, value);
    }
}
```

**使用附加属性**

```xml
<Grid>
    <TextBlock Grid.Row="0" Grid.Column="0" Text="第一行第一列"/>
</Grid>
```

### 11.5 属性值优先级

从高到低：
1. 强制值 (Coerce)
2. 动画
3. 本地值 (Local)
4. 模板触发器
5. 模板绑定
6. 样式触发器
7. 样式设置器
8. 默认值

### 11.6 属性变更通知

```csharp
// 监听任意依赖属性的变更
DependencyPropertyDescriptor
    .FromProperty(UIElement.VisibilityProperty, typeof(UIElement))
    .AddHandler(myElement, OnVisibilityChanged);

private void OnVisibilityChanged(object? sender, EventArgs e)
{
    // 处理可见性变更
}
```

---

## 12. MVVM 模式

### 12.1 MVVM 架构

MVVM（Model-View-ViewModel）是一种将 UI 与业务逻辑分离的设计模式：

- **Model**：数据模型和业务逻辑
- **View**：UI 层（JALXAML）
- **ViewModel**：连接 Model 和 View 的中介

### 12.2 ViewModel 基类

```csharp
using System.ComponentModel;
using System.Runtime.CompilerServices;

public abstract class ViewModelBase : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;
    
    protected virtual void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
    
    protected bool SetProperty<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
            return false;
        
        field = value;
        OnPropertyChanged(propertyName);
        return true;
    }
}
```

### 12.3 创建 ViewModel

```csharp
public class MainViewModel : ViewModelBase
{
    private string _name = "";
    private int _age;
    private bool _isLoading;
    
    public string Name
    {
        get => _name;
        set => SetProperty(ref _name, value);
    }
    
    public int Age
    {
        get => _age;
        set => SetProperty(ref _age, value);
    }
    
    public bool IsLoading
    {
        get => _isLoading;
        set => SetProperty(ref _isLoading, value);
    }
    
    // 命令
    public ICommand SaveCommand { get; }
    public ICommand LoadCommand { get; }
    
    public MainViewModel()
    {
        SaveCommand = new RelayCommand(Save, CanSave);
        LoadCommand = new AsyncRelayCommand(LoadAsync);
    }
    
    private void Save()
    {
        // 保存逻辑
    }
    
    private bool CanSave()
    {
        return !string.IsNullOrWhiteSpace(Name) && Age > 0;
    }
    
    private async Task LoadAsync()
    {
        IsLoading = true;
        try
        {
            // 异步加载
            await Task.Delay(1000);
            Name = "已加载的数据";
        }
        finally
        {
            IsLoading = false;
        }
    }
}
```

### 12.4 RelayCommand

```csharp
public class RelayCommand : ICommand
{
    private readonly Action _execute;
    private readonly Func<bool>? _canExecute;
    
    public event EventHandler? CanExecuteChanged;
    
    public RelayCommand(Action execute, Func<bool>? canExecute = null)
    {
        _execute = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }
    
    public bool CanExecute(object? parameter) => _canExecute?.Invoke() ?? true;
    
    public void Execute(object? parameter) => _execute();
    
    public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);
}

public class RelayCommand<T> : ICommand
{
    private readonly Action<T?> _execute;
    private readonly Func<T?, bool>? _canExecute;
    
    public event EventHandler? CanExecuteChanged;
    
    public RelayCommand(Action<T?> execute, Func<T?, bool>? canExecute = null)
    {
        _execute = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }
    
    public bool CanExecute(object? parameter) => _canExecute?.Invoke((T?)parameter) ?? true;
    
    public void Execute(object? parameter) => _execute((T?)parameter);
    
    public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);
}
```

### 12.5 异步命令

```csharp
public class AsyncRelayCommand : ICommand
{
    private readonly Func<Task> _execute;
    private readonly Func<bool>? _canExecute;
    private bool _isExecuting;
    
    public event EventHandler? CanExecuteChanged;
    
    public bool IsExecuting
    {
        get => _isExecuting;
        private set
        {
            _isExecuting = value;
            CanExecuteChanged?.Invoke(this, EventArgs.Empty);
        }
    }
    
    public AsyncRelayCommand(Func<Task> execute, Func<bool>? canExecute = null)
    {
        _execute = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }
    
    public bool CanExecute(object? parameter) => !IsExecuting && (_canExecute?.Invoke() ?? true);
    
    public async void Execute(object? parameter)
    {
        if (!CanExecute(parameter))
            return;
        
        try
        {
            IsExecuting = true;
            await _execute();
        }
        finally
        {
            IsExecuting = false;
        }
    }
}
```

### 12.6 绑定 ViewModel

**在 Code-Behind 中设置**

```csharp
public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        DataContext = new MainViewModel();
    }
}
```

**在 JALXAML 中绑定命令**

```xml
<Window xmlns:vm="clr-namespace:MyApp.ViewModels">
    <Window.DataContext>
        <vm:MainViewModel/>
    </Window.DataContext>
    
    <StackPanel Margin="20">
        <TextBox Text="{Binding Name, Mode=TwoWay, UpdateSourceTrigger=PropertyChanged}"
                 Watermark="请输入姓名"/>
        
        <TextBox Text="{Binding Age, Mode=TwoWay}"
                 Watermark="请输入年龄" Margin="0,8,0,0"/>
        
        <Button Content="保存"
                Command="{Binding SaveCommand}"
                Margin="0,16,0,0"/>
        
        <Button Content="加载"
                Command="{Binding LoadCommand}"
                IsEnabled="{Binding IsLoading, Converter={StaticResource InverseBooleanConverter}}"
                Margin="0,8,0,0"/>
    </StackPanel>
</Window>
```

### 12.7 集合绑定

```csharp
public class MainViewModel : ViewModelBase
{
    public ObservableCollection<Item> Items { get; } = new();
    
    public MainViewModel()
    {
        LoadItems();
    }
    
    private void LoadItems()
    {
        Items.Add(new Item { Name = "项目 1" });
        Items.Add(new Item { Name = "项目 2" });
    }
}

public class Item : ViewModelBase
{
    private string _name = "";
    public string Name
    {
        get => _name;
        set => SetProperty(ref _name, value);
    }
}
```

```xml
<ListBox ItemsSource="{Binding Items}"
         SelectedItem="{Binding SelectedItem}">
    <ListBox.ItemTemplate>
        <DataTemplate>
            <TextBlock Text="{Binding Name}"/>
        </DataTemplate>
    </ListBox.ItemTemplate>
</ListBox>
```

---

## 13. 窗口和应用程序生命周期

### 13.1 Window 类

```csharp
public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        
        // 设置窗口属性
        Title = "我的窗口";
        Width = 800;
        Height = 600;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
    }
}
```

### 13.2 窗口属性

```xml
<Window Title="窗口标题"
        Width="800"
        Height="600"
        MinWidth="400"
        MinHeight="300"
        MaxWidth="1920"
        MaxHeight="1080"
        WindowState="Normal"
        WindowStyle="SingleBorderWindow"
        ResizeMode="CanResize"
        Topmost="False"
        WindowStartupLocation="CenterScreen"
        Icon="Images/icon.ico">
    <!-- 内容 -->
</Window>
```

### 13.3 自定义标题栏

```xml
<Window TitleBarStyle="Custom"
        TitleBarHeight="32"
        IsShowIcon="True"
        IsShowTitle="True"
        SystemBackdrop="Mica">
    
    <Window.LeftWindowCommands>
        <StackPanel Orientation="Horizontal">
            <Button Content="菜单" Style="{StaticResource TitleBarButtonStyle}"/>
        </StackPanel>
    </Window.LeftWindowCommands>
    
    <Window.RightWindowCommands>
        <StackPanel Orientation="Horizontal">
            <Button Content="设置" Style="{StaticResource TitleBarButtonStyle}"/>
        </StackPanel>
    </Window.RightWindowCommands>
    
    <!-- 主内容 -->
</Window>
```

### 13.4 窗口事件

```csharp
public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        
        Loaded += OnLoaded;
        Closing += OnClosing;
        Closed += OnClosed;
        LocationChanged += OnLocationChanged;
        SizeChanged += OnSizeChanged;
        StateChanged += OnStateChanged;
    }
    
    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        // 窗口加载完成
    }
    
    private void OnClosing(object? sender, CancelEventArgs e)
    {
        // 窗口即将关闭
        // e.Cancel = true; // 取消关闭
    }
    
    private void OnClosed(object? sender, EventArgs e)
    {
        // 窗口已关闭
    }
    
    private void OnLocationChanged(object? sender, EventArgs e)
    {
        // 窗口位置改变
    }
    
    private void OnStateChanged(object? sender, EventArgs e)
    {
        // 窗口状态改变（最大化/最小化/正常）
    }
}
```

### 13.5 对话框

```csharp
// 模态对话框
var dialog = new SettingsWindow();
dialog.Owner = this;
if (dialog.ShowDialog() == true)
{
    // 用户点击确定
    ApplySettings(dialog.Settings);
}

// 非模态窗口
var window = new DetailsWindow();
window.Show();
```

### 13.6 应用程序关闭模式

```csharp
public class Program
{
    public static void Main()
    {
        var app = new Application();
        
        // 关闭模式
        app.ShutdownMode = ShutdownMode.OnLastWindowClose;  // 最后一个窗口关闭时退出
        // app.ShutdownMode = ShutdownMode.OnMainWindowClose; // 主窗口关闭时退出
        // app.ShutdownMode = ShutdownMode.OnExplicitShutdown; // 显式调用 Shutdown() 时退出
        
        app.Run(new MainWindow());
    }
}
```

---

## 14. 主题系统

### 14.1 内置主题

Jalium.UI 内置亮色和暗色主题：

```csharp
// 应用暗色主题
ThemeManager.ApplyTheme(ThemeVariant.Dark);

// 应用亮色主题
ThemeManager.ApplyTheme(ThemeVariant.Light);
```

### 14.2 自定义强调色

```csharp
// 应用自定义强调色
var accent = Color.FromRgb(0x7C, 0x4D, 0xFF); // 紫色
ThemeManager.ApplyAccent(accent);
```

### 14.3 自定义字体

```csharp
ThemeManager.ApplyTypography(
    display: "Segoe UI",           // 显示字体
    body: "Segoe UI",              // 正文字体
    mono: "Cascadia Mono"          // 等宽字体
);
```

### 14.4 完整品牌主题

```csharp
ThemeManager.ApplyBrandTheme(new BrandThemeOptions
{
    Theme = ThemeVariant.Dark,
    AccentColor = Color.FromRgb(0x7C, 0x4D, 0xFF),
    DisplayFontFamily = "Segoe UI",
    BodyFontFamily = "Segoe UI",
    MonoFontFamily = "Cascadia Mono"
});
```

### 14.5 主题资源

在 JALXAML 中使用主题资源：

```xml
<Button Background="{DynamicResource AccentBrush}"
        Foreground="{DynamicResource TextPrimary}"/>

<Border Background="{DynamicResource ControlBackground}"
        BorderBrush="{DynamicResource ControlBorder}">
    <!-- 内容 -->
</Border>
```

**常用主题资源**

| 资源名 | 说明 |
|---|---|
| `AccentBrush` | 强调色画刷 |
| `TextPrimary` | 主要文本颜色 |
| `TextSecondary` | 次要文本颜色 |
| `TextDisabled` | 禁用文本颜色 |
| `ControlBackground` | 控件背景 |
| `ControlBackgroundHover` | 悬停背景 |
| `ControlBackgroundPressed` | 按下背景 |
| `ControlBackgroundDisabled` | 禁用背景 |
| `ControlBorder` | 控件边框 |

---

## 15. 自定义控件开发

### 15.1 创建自定义控件

```csharp
// 定义控件
public class RatingControl : Control
{
    static RatingControl()
    {
        // 重写默认样式键
        DefaultStyleKeyProperty.OverrideMetadata(
            typeof(RatingControl),
            new PropertyMetadata(typeof(RatingControl))
        );
    }
    
    // 注册依赖属性
    public static readonly DependencyProperty ValueProperty =
        DependencyProperty.Register(
            nameof(Value),
            typeof(int),
            typeof(RatingControl),
            new PropertyMetadata(0, OnValueChanged, CoerceValue)
        );
    
    public static readonly DependencyProperty MaximumProperty =
        DependencyProperty.Register(
            nameof(Maximum),
            typeof(int),
            typeof(RatingControl),
            new PropertyMetadata(5)
        );
    
    public int Value
    {
        get => (int)GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }
    
    public int Maximum
    {
        get => (int)GetValue(MaximumProperty);
        set => SetValue(MaximumProperty, value);
    }
    
    private static void OnValueChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        var control = (RatingControl)d;
        // 响应值变化
    }
    
    private static object CoerceValue(DependencyObject d, object baseValue)
    {
        var control = (RatingControl)d;
        var value = (int)baseValue;
        return Math.Max(0, Math.Min(control.Maximum, value));
    }
}
```

### 15.2 控件默认样式

在 `Themes/Generic.jalxaml` 中：

```xml
<Style TargetType="local:RatingControl">
    <Setter Property="Template">
        <Setter.Value>
            <ControlTemplate TargetType="local:RatingControl">
                <StackPanel Orientation="Horizontal" x:Name="PART_Container">
                    <!-- 星星图标 -->
                </StackPanel>
            </ControlTemplate>
        </Setter.Value>
    </Setter>
</Style>
```

### 15.3 模板部件

```csharp
[TemplatePart(Name = "PART_Container", Type = typeof(StackPanel))]
public class RatingControl : Control
{
    private StackPanel? _container;
    
    public override void OnApplyTemplate()
    {
        base.OnApplyTemplate();
        
        // 获取模板部件
        _container = GetTemplateChild("PART_Container") as StackPanel;
        
        if (_container != null)
        {
            // 初始化部件
            UpdateStars();
        }
    }
    
    private void UpdateStars()
    {
        // 更新星星显示
    }
}
```

### 15.4 自定义路由事件

```csharp
public class RatingControl : Control
{
    // 注册路由事件
    public static readonly RoutedEvent ValueChangedEvent =
        EventManager.RegisterRoutedEvent(
            "ValueChanged",
            RoutingStrategy.Bubble,
            typeof(RoutedPropertyChangedEventHandler<int>),
            typeof(RatingControl)
        );
    
    public event RoutedPropertyChangedEventHandler<int> ValueChanged
    {
        add => AddHandler(ValueChangedEvent, value);
        remove => RemoveHandler(ValueChangedEvent, value);
    }
    
    protected virtual void OnValueChanged(int oldValue, int newValue)
    {
        RaiseEvent(new RoutedPropertyChangedEventArgs<int>(oldValue, newValue, ValueChangedEvent));
    }
}
```

---

## 16. 高级主题

### 16.1 动画系统

```xml
<Storyboard x:Key="FadeInStoryboard">
    <DoubleAnimation Storyboard.TargetProperty="Opacity"
                    From="0" To="1"
                    Duration="0:0:0.3">
        <DoubleAnimation.EasingFunction>
            <CubicEase EasingMode="EaseOut"/>
        </DoubleAnimation.EasingFunction>
    </DoubleAnimation>
</Storyboard>
```

**启动动画**

```csharp
var storyboard = (Storyboard)Resources["FadeInStoryboard"];
storyboard.Begin(myElement);
```

### 16.2 调度器

```csharp
// 在 UI 线程执行
Dispatcher.Invoke(() =>
{
    myTextBlock.Text = "更新文本";
});

// 异步执行
Dispatcher.InvokeAsync(() =>
{
    myTextBlock.Text = "异步更新";
});

// 延迟执行
Dispatcher.DelayInvoke(() =>
{
    myTextBlock.Text = "延迟更新";
}, TimeSpan.FromSeconds(1));
```

### 16.3 热重载

Jalium.UI 支持运行时热重载 JALXAML 文件：

```csharp
// 启用热重载（仅调试模式）
#if DEBUG
HotReloadRuntime.Enabled = true;
#endif
```

### 16.4 性能优化

**虚拟化**

```xml
<ListBox VirtualizingPanel.IsVirtualizing="True"
         VirtualizingPanel.VirtualizationMode="Recycling">
    <!-- 大量数据项 -->
</ListBox>
```

**延迟加载**

```xml
<TabControl>
    <TabItem Header="快速加载">
        <TextBlock Text="立即加载"/>
    </TabItem>
    
    <TabItem Header="延迟加载">
        <TabItem.ContentTemplate>
            <DataTemplate>
                <ContentControl Content="{Binding}"/>
            </DataTemplate>
        </TabItem.ContentTemplate>
    </TabItem>
</TabControl>
```

**Freezable**

```csharp
// 冻结画刷以提高性能
var brush = new SolidColorBrush(Colors.Red);
brush.Freeze();
```

---

## 附录

### A. 命名空间映射

| 命名空间 | 前缀示例 | 说明 |
|---|---|---|
| `http://schemas.microsoft.com/winfx/2006/xaml/presentation` | 默认 | 控件和基础类型 |
| `http://schemas.microsoft.com/winfx/2006/xaml` | x | XAML 语言特性 |
| `clr-namespace:MyApp` | local | 本地命名空间 |
| `clr-namespace:MyApp.Controls;assembly=MyApp.Controls` | controls | 外部程序集 |

### B. 常用类型转换

| 类型 | 字符串表示 |
|---|---|
| `Color` | `#FF0000`, `Red`, `#80FF0000` |
| `Brush` | `Red`, `#FF0000`, `Transparent` |
| `Thickness` | `10`, `10,20`, `10,20,30,40` |
| `GridLength` | `Auto`, `*`, `2*`, `100` |
| `Visibility` | `Visible`, `Collapsed`, `Hidden` |
| `HorizontalAlignment` | `Left`, `Center`, `Right`, `Stretch` |
| `VerticalAlignment` | `Top`, `Center`, `Bottom`, `Stretch` |
| `Orientation` | `Horizontal`, `Vertical` |
| `CornerRadius` | `4`, `4,8`, `4,8,12,16` |

### C. 键盘快捷键

```xml
<!-- 定义快捷键 -->
<Window.InputBindings>
    <KeyBinding Key="S" Modifiers="Ctrl" Command="{Binding SaveCommand}"/>
    <KeyBinding Key="O" Modifiers="Ctrl" Command="{Binding OpenCommand}"/>
    <KeyBinding Key="N" Modifiers="Ctrl" Command="{Binding NewCommand}"/>
</Window.InputBindings>

<!-- 定义命令绑定 -->
<Window.CommandBindings>
    <CommandBinding Command="ApplicationCommands.Save" Executed="OnSave"/>
</Window.CommandBindings>
```

### D. 资源参考

- **GitHub 仓库**: https://github.com/VeryJokerJal/Jalium.UI
- **问题反馈**: https://github.com/VeryJokerJal/Jalium.UI/issues
- **许可证**: MIT

---

*文档版本: 1.0*
*最后更新: 2026年3月*
