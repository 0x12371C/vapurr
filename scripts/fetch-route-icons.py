"""Vendor pinned brand SVGs for offline token icons; never executes downloaded content."""
from pathlib import Path
from urllib.request import urlopen
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
ICONS = {'amzn':'amazon','tsla':'tesla','amd':'amd','nflx':'netflix','pltr':'palantir','nvda':'nvidia'}
for ticker, slug in ICONS.items():
    url = f'https://cdn.jsdelivr.net/npm/simple-icons@11.15.0/icons/{slug}.svg'
    data = urlopen(url, timeout=20).read(100_000)
    svg = ET.fromstring(data)
    assert svg.tag == '{http://www.w3.org/2000/svg}svg'
    assert all(node.tag.rsplit('}',1)[-1] in {'svg','title','path'} for node in svg.iter())
    assert all(not key.lower().startswith('on') and 'href' not in key.lower() for node in svg.iter() for key in node.attrib)
    (ROOT/'frontend/tokens'/f'{ticker}.svg').write_bytes(data)
    print(ticker, len(data), url)
