"""Exercise the real Rust token catalog, native fetch wrapper and complete swap page."""
import json
import mimetypes
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlparse, unquote
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
FRONT = ROOT / 'frontend'
OUT = ROOT / 'dist' / 'route-preview'

def main():
    command = [str(ROOT/'target/debug/examples/route_catalog.exe')]
    if '--verify' in sys.argv: command.append('--verify')
    result = subprocess.run(command, capture_output=True, text=True, check=True)
    catalog = json.loads(result.stdout)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT/'catalog.json').write_text(json.dumps(catalog, indent=2), encoding='utf-8')
    security = (ROOT/'crates/vapurr-shell/src/security.js').read_text(encoding='utf-8').replace('__API_TOKEN__', '"test-capability"')
    with sync_playwright() as p:
        browser = p.chromium.launch(channel='msedge', headless=True)
        context = browser.new_context(viewport={'width':1100,'height':1000}, reduced_motion='reduce')
        errors = []
        requests = []
        reject_catalog = [False]
        def serve(route):
            url = urlparse(route.request.url)
            if url.path == '/route/api/tokens':
                requests.append(route.request.headers)
                if reject_catalog[0] or route.request.headers.get('x-vapurr-client') != 'test-capability':
                    route.fulfill(status=403, json={'ok':False,'error':'Untrusted API caller'})
                else: route.fulfill(json=catalog)
                return
            if '/api/' in url.path:
                route.fulfill(json={'ok':False,'error':'No transaction in UI check'})
                return
            path = (FRONT/unquote(url.path).lstrip('/')).resolve()
            if path.is_relative_to(FRONT) and path.is_file():
                if path.name == 'swap.html':
                    html = path.read_text(encoding='utf-8').replace('</head>', '<script type="application/json" id="route-catalog">'+json.dumps(catalog).replace('<', '\\u003c')+'</script></head>')
                    route.fulfill(body=html, content_type='text/html')
                    return
                route.fulfill(path=str(path), content_type=mimetypes.guess_type(path)[0] or 'application/octet-stream')
            else: route.fulfill(status=404, body='Not found')
        context.route('http://vapurr.localhost/**', serve)
        context.route('https://**', lambda route: route.abort())
        context.add_init_script(security)
        context.add_init_script('window.__messages=[]; window.ipc={postMessage:s=>__messages.push(JSON.parse(s))};')
        page = context.new_page()
        page.on('pageerror', lambda e: errors.append(str(e)))
        page.goto('http://vapurr.localhost/swap.html')
        page.evaluate("__setWallet({chain_id:46630,address:'0x'+'1'.repeat(40),assets:[],eth:'1'})")
        page.wait_for_function("document.querySelector('#from-tok').textContent.includes('wgV')")
        page.locator('#from-tok').click()
        rows = page.locator('#tok-list .tok-row')
        assert rows.count() >= 9, rows.all_text_contents()
        assert requests and requests[0].get('x-vapurr-client') == 'test-capability'
        assert 'Testnet' in page.locator('#route-network').inner_text()
        page.wait_for_function("Array.from(document.querySelectorAll('#tok-list img')).every(i=>i.complete && i.naturalWidth>0)")
        assert page.locator('#tok-list img').count() == rows.count()
        for theme in ['dark','light']:
            page.evaluate('(t)=>document.documentElement.dataset.theme=t', theme)
            page.screenshot(path=str(OUT/f'tokens-{theme}.png'), full_page=True)
        # A transient API/auth failure cannot blank a locally supplied catalog.
        reject_catalog[0] = True
        page.reload()
        page.wait_for_function("document.querySelector('#from-tok').textContent.includes('wgV')")
        page.locator('#from-tok').click()
        assert page.locator('#tok-list .tok-row').count() >= 9
        assert not errors, errors
        print('PASS: authenticated API, deployed testnet picker:', ', '.join(t['symbol'] for t in catalog['tokens']))
        if '--verify' in sys.argv:
            print(json.dumps([{k:v for k,v in t.items() if k in ['symbol','address','verified','decimals_rpc','symbol_rpc']} for t in catalog['tokens']], indent=2))
        browser.close()

if __name__ == '__main__': main()
