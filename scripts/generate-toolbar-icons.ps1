# Original vector glyphs rasterized into transparent, multi-resolution Windows
# icons. Uses the installed Windows drawing library; no app build or packages.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$iconDirectory = Join-Path $taskRoot 'crates/win32/resources/icons'
[void][System.IO.Directory]::CreateDirectory($iconDirectory)

function Add-Polygon($path, [float[]]$coordinates) {
    $points = [System.Collections.Generic.List[System.Drawing.PointF]]::new()
    for ($i = 0; $i -lt $coordinates.Length; $i += 2) {
        $points.Add([System.Drawing.PointF]::new($coordinates[$i], $coordinates[$i + 1]))
    }
    $path.AddPolygon($points.ToArray())
}

function New-IconFrame([string]$name, [int]$size, [bool]$disabled) {
    $bitmap = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear([System.Drawing.Color]::Transparent)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.ScaleTransform(($size / 32.0), ($size / 32.0))
    $colors = @{ add = '#70B99F'; resume = '#8EB9AB'; stop = '#9D9D9D'; stop_all = '#A0A0A0'; delete = '#AAAAAA'; delete_completed = '#AE72AC'; options = '#8270D4'; scheduler = '#AD81B2'; start_queue = '#83A2CD'; stop_queue = '#9B83CA'; tell_friend = '#A08BD4' }
    $color = [System.Drawing.ColorTranslator]::FromHtml($(if ($disabled) { '#BFBFBF' } else { $colors[$name] }))
    $pen = [System.Drawing.Pen]::new($color, 1.55)
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $fill = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(18, $color))
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    try {
        switch ($name) {
            'add' { Add-Polygon $path @(12,3, 20,3, 20,12, 29,12, 29,20, 20,20, 20,29, 12,29, 12,20, 3,20, 3,12, 12,12) }
            'resume' {
                $path.AddEllipse(3,3,26,26)
                $path.StartFigure(); $path.AddLine(16,8,16,23)
                $path.StartFigure(); $path.AddLine(10,17,16,23); $path.AddLine(16,23,22,17)
            }
            'delete_completed' {
                $path.AddRectangle([System.Drawing.RectangleF]::new(5,7,22,23))
                $path.StartFigure(); $path.AddLine(3,7,29,7)
                $path.StartFigure(); $path.AddBezier(11,6,11,0,21,0,21,6)
                $path.StartFigure(); $path.AddLine(10,17,14,22); $path.AddLine(14,22,23,12)
            }
            'scheduler' {
                $path.AddEllipse(5,5,22,24)
                $path.StartFigure(); $path.AddLine(16,10,16,18); $path.AddLine(16,18,21,14)
                $path.StartFigure(); $path.AddBezier(4,10,-2,5,6,-1,10,4)
                $path.StartFigure(); $path.AddBezier(22,4,27,-1,34,6,28,10)
                $path.StartFigure(); $path.AddLine(8,27,6,30)
                $path.StartFigure(); $path.AddLine(24,27,26,30)
            }
            'start_queue' {
                $path.AddLine(22,3,4,3); $path.AddLine(4,3,4,25); $path.AddLine(4,25,10,25)
                $path.StartFigure(); $path.AddLine(27,7,9,7); $path.AddLine(9,7,9,29); $path.AddLine(9,29,17,29)
                $path.StartFigure(); $path.AddLine(23,13,23,29)
                $path.StartFigure(); $path.AddLine(17,23,23,29); $path.AddLine(23,29,29,23)
            }
            'stop_queue' {
                $path.AddLine(22,3,4,3); $path.AddLine(4,3,4,25); $path.AddLine(4,25,10,25)
                $path.StartFigure(); $path.AddLine(27,7,9,7); $path.AddLine(9,7,9,29); $path.AddLine(9,29,16,29)
                $path.StartFigure(); $path.AddRectangle([System.Drawing.RectangleF]::new(20,18,11,12))
            }
            'tell_friend' {
                $path.AddRectangle([System.Drawing.RectangleF]::new(2,13,6,16))
                $path.StartFigure(); $path.AddLine(8,14,14,8)
                $path.AddBezier(14,8,16,6,13,1,17,1)
                $path.AddBezier(17,1,21,1,23,9,21,12)
                $path.AddLine(21,12,28,12); $path.AddBezier(28,12,31,12,30,17,29,19)
                $path.AddLine(29,19,27,28); $path.AddLine(27,28,15,28)
                $path.AddLine(15,28,8,25); $path.AddLine(8,25,8,14)
            }
            'stop' {
                $path.AddEllipse(3, 3, 26, 26)
                $path.StartFigure(); $path.AddRectangle([System.Drawing.RectangleF]::new(11,11,10,10))
            }
            'stop_all' {
                $path.AddLine(11,28,7,23); $path.AddLine(7,23,3,17)
                $path.AddBezier(3,17,1,14,4,12,6,14); $path.AddLine(6,14,10,18)
                $path.AddLine(10,18,10,7); $path.AddBezier(10,7,10,4,14,4,14,7)
                $path.AddLine(14,7,14,3); $path.AddBezier(14,3,14,0,18,0,18,3)
                $path.AddLine(18,3,18,7); $path.AddBezier(18,7,18,4,22,4,22,7)
                $path.AddLine(22,7,22,10); $path.AddBezier(22,10,22,7,26,7,26,10)
                $path.AddLine(26,10,26,19); $path.AddBezier(26,19,26,25,23,29,18,29)
                $path.AddLine(18,29,11,28); $path.CloseFigure()
            }
            'delete' {
                Add-Polygon $path @(7,9, 25,9, 23,28, 9,28)
                $path.StartFigure(); $path.AddLine(5,6,27,6)
                $path.StartFigure(); $path.AddBezier(12,6,12,0,20,0,20,6)
                $path.StartFigure(); $path.AddLine(13,13,13,23)
                $path.StartFigure(); $path.AddLine(19,13,19,23)
            }
            'options' {
                $points = [System.Collections.Generic.List[System.Drawing.PointF]]::new()
                for ($tooth = 0; $tooth -lt 8; $tooth++) {
                    foreach ($vertex in @(@(-21,10.5), @(-11,14), @(11,14), @(21,10.5))) {
                        $angle = ($tooth * 45 + $vertex[0]) * [Math]::PI / 180
                        $points.Add([System.Drawing.PointF]::new((16 + $vertex[1] * [Math]::Cos($angle)), (16 + $vertex[1] * [Math]::Sin($angle))))
                    }
                }
                $path.AddPolygon($points.ToArray()); $path.StartFigure(); $path.AddEllipse(10.5,10.5,11,11)
            }
        }
        $graphics.FillPath($fill, $path)
        $graphics.DrawPath($pen, $path)
    }
    finally { $path.Dispose(); $pen.Dispose(); $fill.Dispose(); $graphics.Dispose() }
    return $bitmap
}

$names = @('add', 'resume', 'stop', 'stop_all', 'delete', 'delete_completed', 'options', 'scheduler', 'start_queue', 'stop_queue', 'tell_friend')
$sizes = @(24, 30, 32, 36, 40, 48, 56, 64, 80, 96)
foreach ($name in $names) {
    foreach ($disabled in @($false, $true)) {
        $frames = [System.Collections.Generic.List[byte[]]]::new()
        foreach ($size in $sizes) {
            $bitmap = New-IconFrame $name $size $disabled
            $stream = [System.IO.MemoryStream]::new()
            try { $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png); $frames.Add($stream.ToArray()) }
            finally { $stream.Dispose(); $bitmap.Dispose() }
        }
        $suffix = if ($disabled) { '-disabled' } else { '' }
        $file = [System.IO.File]::Create((Join-Path $iconDirectory "$name$suffix.ico"))
        $writer = [System.IO.BinaryWriter]::new($file)
        try {
            $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Length)
            $offset = 6 + 16 * $sizes.Length
            for ($i = 0; $i -lt $sizes.Length; $i++) {
                $writer.Write([byte]$sizes[$i]); $writer.Write([byte]$sizes[$i])
                $writer.Write([byte]0); $writer.Write([byte]0)
                $writer.Write([uint16]1); $writer.Write([uint16]32)
                $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
                $offset += $frames[$i].Length
            }
            foreach ($frame in $frames) { $writer.Write($frame) }
        }
        finally { $writer.Dispose() }
    }
}

# Asset preview only: this does not launch or screenshot the application.
$preview = [System.Drawing.Bitmap]::new(905, 170)
$graphics = [System.Drawing.Graphics]::FromImage($preview)
$font = [System.Drawing.Font]::new('Tahoma', 8)
$brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(55,55,55))
try {
    $graphics.Clear([System.Drawing.Color]::FromArgb(247,247,247))
    $labels = @('Add URL', 'Resume', 'Stop', 'Stop All', 'Delete', 'Delete Co...', 'Options', 'Scheduler', 'Start Qu...', 'Stop Qu...', 'Tell a Fri...')
    for ($row = 0; $row -lt 2; $row++) {
        for ($i = 0; $i -lt $names.Length; $i++) {
            $bitmap = New-IconFrame $names[$i] 32 ($row -eq 1 -or $i -in @(1,2,3,4))
            try { $graphics.DrawImageUnscaled($bitmap, 22 + $i * 80, 12 + $row * 80) }
            finally { $bitmap.Dispose() }
            $labelSize = $graphics.MeasureString($labels[$i], $font)
            $graphics.DrawString($labels[$i], $font, $brush, (38 + $i * 80 - $labelSize.Width / 2), (52 + $row * 80))
        }
    }
    $preview.Save((Join-Path $taskRoot 'docs/design/rusty-toolbar-icons.png'), [System.Drawing.Imaging.ImageFormat]::Png)
}
finally { $brush.Dispose(); $font.Dispose(); $graphics.Dispose(); $preview.Dispose() }
Write-Output 'Generated eleven original toolbar icons plus disabled variants, each at 24/30/32/36/40/48/56/64/80/96 px.'
