"""Offline Edge smoke checks for the DeFi desks; never connects a wallet or RPC."""
from pathlib import Path
from urllib.parse import urlparse, unquote
import json
import mimetypes
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
FRONT = ROOT / 'frontend'
OUT = ROOT / 'dist' / 'defi-preview'
SNAP = dict(live=True, net='testnet', status='live', price='0.05', apy='9',
            vapurr='24500', pusd='1280.45', eth='0.1', index='1.0231',
            yield_reserve='500', pusd_supply='42000', min_spread='2',
            market='0x'+'1'*40, address='0x'+'2'*40,
            loop=dict(live=True, supplied='1200', debt='400', cash='8000',
                      collat_v='12000', collat_value='1800', total_assets='12000',
                      util='33.33', borrow_apy='6', supply_apy='2', max_ltv='8500', health='4.05'),
            house=dict(live=True, need_deploy=False, vapurr='1400', pusd='950',
                       house='0x'+'3'*40, pool_id='0x'+'4'*64, token_id='7'))

def main():
    OUT.mkdir(parents=True, exist_ok=True)
    results = []
    with sync_playwright() as p:
        browser = p.chromium.launch(channel='msedge', headless=True)
        context = browser.new_context(viewport={'width':1440,'height':1080}, reduced_motion='reduce')
        def serve(route):
            path = (FRONT / unquote(urlparse(route.request.url).path).lstrip('/')).resolve()
            if not path.is_relative_to(FRONT) or not path.is_file():
                route.fulfill(status=404, body='Not found')
            else:
                mime = mimetypes.guess_type(path)[0] or 'application/octet-stream'
                route.fulfill(path=str(path), content_type=mime)
        context.route('http://vapurr.localhost/**', serve)
        context.route('https://**', lambda route: route.abort())
        context.add_init_script('window.__messages=[]; window.ipc={postMessage: s => window.__messages.push(JSON.parse(s))};')
        for name in ['defi','pusd','bonds']:
            page = context.new_page()
            errors=[]
            page.on('pageerror', lambda err: errors.append(str(err)))
            page.goto('http://vapurr.localhost/'+name+'.html', wait_until='load')
            page.evaluate('document.fonts.ready')
            if name=='pusd':
                assert page.locator('#boot').is_visible()
                page.evaluate('(s)=>window.__setEcon(s)', SNAP)
                assert page.locator('#book').is_visible()
                assert page.evaluate('fmtBag(-25.5)')=='-25.50'
                assert page.locator('#bal-p').inner_text()=='2,080.45'
                page.locator('[data-mode=redeem]').click()
                assert page.locator('#book .mini-route > span').first.inner_text()=='PUSD'
                page.locator('[data-mode=mint]').click()
                page.locator('#amt').fill('10')
                assert 'PUSD' in page.locator('#quote').inner_text()
                # Entering an amount must not send a transaction.
                assert not page.evaluate("window.__messages.some(m=>m.cmd==='econ-mint')")
                # Review and rejection must never broadcast.
                page.locator('#go').click()
                assert page.locator('#vs-title').inner_text()=='Mint $PUSD'
                assert '0.49' in page.locator('#vs-rows').inner_text()
                page.locator('#vs-no').click()
                assert not page.evaluate("window.__messages.some(m=>m.cmd==='econ-mint')")
                # Accept only into the mocked IPC sink, then complete the fake receipt.
                page.locator('#go').click()
                page.locator('#vs-go').click()
                page.wait_for_function("window.__messages.some(m=>m.cmd==='econ-mint' && m.amt==='10')")
                page.evaluate('() => { vapurr.finishTx(true,{tx:"0xui_test",tx_status:"confirmed"}); }')
                page.locator('#vs-no').click()
                page.locator('[data-jump=euler]').click()
                assert not page.locator('#book').is_visible()
                for mode in ['borrow','loop','v','supply']:
                    page.locator('[data-emode='+mode+']').click()
                page.locator('#e-ltv').evaluate("el=>el.textContent='42.5%'")
                page.locator('#e-maxltv').evaluate("el=>el.textContent='85%'")
                page.wait_for_function("document.querySelector('.meter-fill').style.width==='50%'")
            if name=='defi':
                for flow,engine,url in [('mint','Lithe','vapurr://lithe'),('supply','Oliver','vapurr://oliver'),('borrow','Oliver','vapurr://oliver'),('bond','Bonds','vapurr://bonds'),('trade','House','vapurr://house')]:
                    page.locator('[data-flow='+flow+']').click()
                    assert page.locator('#flow-engine').inner_text()==engine
                    page.locator('#flow-open').click()
                    assert page.evaluate('window.__messages.at(-1).url')==url
                page.locator('[data-flow=mint]').click()
            if name=='bonds':
                page.locator('[data-asset=USDG]').click()
                assert page.locator('#q-asset').inner_text()=='USDG'
                page.locator('[data-asset=USDG]').press('ArrowRight')
                assert page.locator('#q-asset').inner_text()=='AMZN'
                page.locator('#bond-cta').click()
                assert page.locator('#bond-err').is_visible()
                assert not page.evaluate("window.__messages.some(m=>m.cmd==='econ-bond')")
                page.locator('[data-asset=ETH]').click()
            for theme in ['dark','light']:
                page.evaluate('(theme)=>document.documentElement.dataset.theme=theme',theme)
                for width in [1440,768,390]:
                    page.set_viewport_size({'width':width,'height':1080 if width>400 else 844})
                    page.evaluate('window.scrollTo(0,0)')
                    if name=='pusd':
                        for panel in ['book','euler','house']:
                            page.locator('[data-jump='+panel+']').click()
                            page.evaluate('window.scrollTo(0,0)')
                            page.screenshot(path=str(OUT/f'{name}-{panel}-{theme}-{width}.png'), full_page=True)
                            assert not page.evaluate('document.documentElement.scrollWidth>innerWidth'), (panel,width,'overflow')
                        page.locator('[data-jump=book]').click()
                        page.evaluate('window.scrollTo(0,0)')
                    page.screenshot(path=str(OUT/f'{name}-{theme}-{width}.png'), full_page=True)
                    overflow=page.evaluate('document.documentElement.scrollWidth>innerWidth')
                    assert not overflow, (name,theme,width,'horizontal overflow')
                    assert not errors, (name,errors)
                    results.append(f'{name} {theme} {width}: OK')
            ids=page.locator('[id]').evaluate_all('els=>els.map(el=>el.id)')
            assert len(ids)==len(set(ids)), (name,'duplicate IDs')
            page.close()
        for query,panel in [('tab=oliver','euler'),('tab=house','house')]:
            page=context.new_page()
            page.goto('http://vapurr.localhost/pusd.html?'+query)
            page.evaluate('(s)=>window.__setEcon(s)', SNAP)
            assert page.locator('#'+panel).is_visible()
            assert not page.locator('#book').is_visible()
            page.close()
        browser.close()
    print('\n'.join(results))
    print('Navigation, form modes, input validation, snapshot rendering and LTV meter: OK')
    print('Screenshots:', OUT)

if __name__=='__main__':
    main()
