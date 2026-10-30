"""把 art-inbox/ 裡 Codex 畫好的 PNG 轉成遊戲用的 webp，放進 src/assets/art/（src/lib/art.js 會自動找到）。

用法：python scripts/import-art.py            # 全部
      python scripts/import-art.py enemies    # 只處理名字含 enemies 的資料夾

art-inbox/（不進 git）的結構：
  costumes/<角色 id>/{normal,happy,win,sad}.png   → costumes/<角色 id>/<表情>.webp（512×512）
  sprites/<角色 id>/{idle,attack}.png             → sprites/<角色 id>/<姿勢>.webp（高 768）
  enemies*/<敵人 id>.png                          → enemies/<敵人 id>.webp（640×640）
  backgrounds/bg_*.png、scenes/*.png              → scenes/<名稱>.webp（寬 1600）
  icon.png                                        → public/favicon.png、favicon.ico（切圓角）
透明背景的圖會先裁掉四周的透明邊再縮放，讓角色在畫面上的大小一致。
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
INBOX = ROOT / 'art-inbox'
OUT = ROOT / 'src' / 'assets' / 'art'
only = sys.argv[1] if len(sys.argv) > 1 else ''


def save(im, path, quality=86):
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path, 'WEBP', quality=quality, method=6)
    print('  ', path.relative_to(ROOT), im.size)


def trimmed(im):
    """裁掉透明邊（沒有透明就原樣）"""
    if im.mode != 'RGBA':
        return im
    box = im.getchannel('A').point(lambda a: 255 if a > 8 else 0).getbbox()
    return im.crop(box) if box else im


def fit(im, w, h):
    """等比縮進 w×h，貼在透明畫布的底部中間（腳對齊底邊）"""
    im = trimmed(im.convert('RGBA'))
    scale = min(w / im.width, h / im.height)
    im = im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))), Image.LANCZOS)
    canvas = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    canvas.paste(im, ((w - im.width) // 2, h - im.height), im)
    return canvas


def wanted(path):
    return only in str(path.relative_to(INBOX)).replace('\\', '/')


for d in sorted((INBOX / 'costumes').glob('*')):
    for png in sorted(d.glob('*.png')):
        if png.stem == 'ref' or not wanted(png):
            continue
        im = Image.open(png).convert('RGBA').resize((512, 512), Image.LANCZOS)
        save(im, OUT / 'costumes' / d.name / f'{png.stem}.webp')

for d in sorted((INBOX / 'sprites').glob('*')):
    for png in sorted(d.glob('*.png')):
        if png.stem == 'ref' or not wanted(png):
            continue
        save(fit(Image.open(png), 576, 768), OUT / 'sprites' / d.name / f'{png.stem}.webp')

for d in sorted(INBOX.glob('enemies*')):
    for png in sorted(d.glob('*.png')):
        if wanted(png):
            save(fit(Image.open(png), 640, 640), OUT / 'enemies' / f'{png.stem}.webp')

for d in [INBOX / 'backgrounds', INBOX / 'scenes']:
    for png in sorted(d.glob('*.png')):
        if wanted(png):
            im = Image.open(png).convert('RGB')
            im = im.resize((1600, round(1600 * im.height / im.width)), Image.LANCZOS)
            save(im, OUT / 'scenes' / f'{png.stem}.webp', quality=82)

icon = INBOX / 'icon.png'
if icon.exists() and wanted(icon):
    im = Image.open(icon).convert('RGBA').resize((512, 512), Image.LANCZOS)
    mask = Image.new('L', im.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((8, 8, 503, 503), radius=96, fill=255)
    im.putalpha(mask)
    pub = ROOT / 'public'
    im.resize((192, 192), Image.LANCZOS).save(pub / 'favicon.png')
    im.save(pub / 'favicon.ico', sizes=[(16, 16), (32, 32), (48, 48)])
    (INBOX / 'icon-rounded.png').parent.mkdir(exist_ok=True)
    im.save(INBOX / 'icon-rounded.png')
    print('   public/favicon.png, favicon.ico')
