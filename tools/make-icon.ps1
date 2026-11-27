# アプリアイコン (icons/icon.png, icons/icon.ico) を生成する
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
$outDir = Join-Path $root 'icons'
New-Item -ItemType Directory -Force $outDir | Out-Null

function New-RoundRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    $p
}

function New-IconPng([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.ScaleTransform($size / 256, $size / 256)

    $bg = New-RoundRect 8 8 240 240 60
    $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.PointF]::new(8, 8)), ([System.Drawing.PointF]::new(248, 248)), ([System.Drawing.Color]::FromArgb(255, 86, 128, 255)), ([System.Drawing.Color]::FromArgb(255, 186, 96, 255))
    $g.FillPath($grad, $bg)

    $shine = New-RoundRect 18 14 220 110 52
    $shineBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.PointF]::new(0, 14)), ([System.Drawing.PointF]::new(0, 124)), ([System.Drawing.Color]::FromArgb(110, 255, 255, 255)), ([System.Drawing.Color]::FromArgb(0, 255, 255, 255))
    $g.FillPath($shineBrush, $shine)
    $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(120, 255, 255, 255)), 3), $bg)

    $white = [System.Drawing.Brushes]::White
    $g.FillPath($white, (New-RoundRect 100 48 56 108 28))
    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::White), 15
    $pen.StartCap = 'Round'; $pen.EndCap = 'Round'
    $g.DrawArc($pen, 72, 84, 112, 104, 0, 180)
    $g.DrawLine($pen, 128, 190, 128, 214)
    $g.DrawLine($pen, 98, 214, 158, 214)

    $g.Dispose()
    $ms = New-Object IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    , $ms.ToArray()
}

[IO.File]::WriteAllBytes((Join-Path $outDir 'icon.png'), (New-IconPng 256))

$sizes = 16, 24, 32, 48, 64, 256
$images = $sizes | ForEach-Object { , (New-IconPng $_) }
$ms = New-Object IO.MemoryStream
$w = New-Object IO.BinaryWriter $ms
$w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $s = $sizes[$i]; $len = $images[$i].Length
    $dim = if ($s -ge 256) { 0 } else { $s }
    $w.Write([byte]$dim); $w.Write([byte]$dim); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([UInt16]1); $w.Write([UInt16]32); $w.Write([UInt32]$len); $w.Write([UInt32]$offset)
    $offset += $len
}
foreach ($img in $images) { $w.Write($img) }
$w.Flush()
[IO.File]::WriteAllBytes((Join-Path $outDir 'icon.ico'), $ms.ToArray())
