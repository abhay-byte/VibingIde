import urllib.request
import os

os.makedirs('assets', exist_ok=True)

fonts = {
    'SpaceGrotesk-Regular.ttf': 'https://github.com/floriankarsten/space-grotesk/raw/master/fonts/ttf/SpaceGrotesk-Regular.ttf',
    'Inter-Regular.ttf': 'https://raw.githubusercontent.com/rsms/inter/master/docs/font-files/Inter-Regular.ttf'
}

for name, url in fonts.items():
    try:
        print(f"Downloading {name}...")
        urllib.request.urlretrieve(url, f"assets/{name}")
        print(f"Success for {name}")
    except Exception as e:
        print(f"Failed to download {name}: {e}")
        # fallback for space grotesk if master is main
        if 'SpaceGrotesk' in name:
            try:
                fallback_url = 'https://github.com/floriankarsten/space-grotesk/raw/main/fonts/ttf/SpaceGrotesk-Regular.ttf'
                print(f"Trying fallback for {name}...")
                urllib.request.urlretrieve(fallback_url, f"assets/{name}")
                print(f"Success for {name}")
            except Exception as e2:
                print(f"Fallback failed: {e2}")
