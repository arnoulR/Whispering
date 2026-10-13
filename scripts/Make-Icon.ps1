$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
# Render at 4x resolution, then filter down to each actual display size.
$master = New-Object Drawing.Bitmap 1024,1024
$g = [Drawing.Graphics]::FromImage($master)
$g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.ScaleTransform(4,4)
$g.Clear([Drawing.Color]::Transparent)
$background = New-Object Drawing.Drawing2D.LinearGradientBrush ([Drawing.Rectangle]::new(8,8,240,240)),([Drawing.Color]::FromArgb(78,136,255)),([Drawing.Color]::FromArgb(30,67,173)),45
$shape = New-Object Drawing.Drawing2D.GraphicsPath
$shape.AddArc(8,8,72,72,180,90)
$shape.AddArc(176,8,72,72,270,90)
$shape.AddArc(176,176,72,72,0,90)
$shape.AddArc(8,176,72,72,90,90)
$shape.CloseFigure()
$g.FillPath($background,$shape)
$pen = New-Object Drawing.Pen ([Drawing.Color]::White),14
$pen.StartCap = $pen.EndCap = [Drawing.Drawing2D.LineCap]::Round
$g.DrawLine($pen,48,114,48,142)
$g.DrawLine($pen,76,90,76,166)
$g.DrawLine($pen,180,90,180,166)
$g.DrawLine($pen,208,114,208,142)
$sparkle = New-Object Drawing.Drawing2D.GraphicsPath
$sparkle.AddBezier(128,66,135,108,141,118,164,128)
$sparkle.AddBezier(164,128,141,138,135,148,128,190)
$sparkle.AddBezier(128,190,121,148,115,138,92,128)
$sparkle.AddBezier(92,128,115,118,121,108,128,66)
$sparkle.CloseFigure()
$g.FillPath([Drawing.Brushes]::White,$sparkle)
$g.Dispose(); $shape.Dispose(); $sparkle.Dispose(); $pen.Dispose(); $background.Dispose()
$sizes = @(16,20,24,32,40,48,64,128,256)
$frames = @()
foreach ($size in $sizes) {
    $bitmap = New-Object Drawing.Bitmap $size,$size
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
    $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.DrawImage($master,[Drawing.Rectangle]::new(0,0,$size,$size),0,0,1024,1024,[Drawing.GraphicsUnit]::Pixel)
    $memory = New-Object IO.MemoryStream
    $bitmap.Save($memory,[Drawing.Imaging.ImageFormat]::Png)
    $frames += ,$memory.ToArray()
    if ($size -eq 256) { $bitmap.Save((Join-Path $root 'assets/whispering.png'),[Drawing.Imaging.ImageFormat]::Png) }
    if ($size -eq 64) { $bitmap.Save((Join-Path $root 'assets/whispering-ui.png'),[Drawing.Imaging.ImageFormat]::Png) }
    $memory.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
}
# PNG-backed ICO preserves alpha and supplies native sizes for Explorer/taskbar.
$stream = [IO.File]::Create((Join-Path $root 'assets/whispering.ico'))
$writer = New-Object IO.BinaryWriter $stream
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $dimension = if ($sizes[$i] -eq 256) { 0 } else { $sizes[$i] }
    $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
    $writer.Write([byte]0); $writer.Write([byte]0)
    $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
    $offset += $frames[$i].Length
}
foreach ($frame in $frames) { $writer.Write([byte[]]$frame) }
$writer.Dispose(); $master.Dispose()
