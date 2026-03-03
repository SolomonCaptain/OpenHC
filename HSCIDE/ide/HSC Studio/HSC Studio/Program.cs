using Jalium.UI.Controls;

var app = new Application();
var window = new Window
{
    Title = "HSC Studio",
    Width = 960,
    Height = 640,
    Content = new StackPanel { /* ... */ }
};
app.Run(window);