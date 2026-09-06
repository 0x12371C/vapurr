"""Offline regression checks against the real route.js. No wallet, RPC, or transaction."""
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
HARNESS = '''<input id="amt"><button id="from-tok"></button><button id="to-tok"></button>
<button id="flip"></button><button id="go"></button><div id="route"></div><div id="receive"></div>
<span id="to-amt"></span><span id="from-bal"></span><span id="to-bal"></span>
<span id="chip-refund"></span><span id="chip-hops"></span><span id="chip-sim"></span>'''
MOCK = '''window.__requests=[]; window.__messages=[]; window.__reviews=[];
window.vapurr={send:m=>{__messages.push(m);return true},beginTx:s=>{__reviews.push(s);return new Promise(r=>window.__approve=r)},finishTx:(ok,s)=>window.__failure=s};
window.fetch=url=>url.includes('/tokens')?Promise.resolve({ok:true,json:()=>Promise.resolve({chains:[],tokens:[
{chain_id:46630,address:'0x1111111111111111111111111111111111111111',symbol:'wgV',decimals:18},
{chain_id:46630,address:'0x2222222222222222222222222222222222222222',symbol:'PUSD',decimals:18}]})}):
new Promise(resolve=>__requests.push({url,resolve}));
window.respond=(i,extra={})=>__requests[i].resolve({ok:true,json:()=>Promise.resolve(Object.assign({
ok:true,payable:true,execution_id:'route-'+i,expires_in_ms:45000,tool:'house',from_chain:46630,to_chain:46630,
from_symbol:'wgV',to_symbol:'PUSD',from_display:'1',to_display:'2',to_min_display:'1.99',sim:{ok:true,ran:true,gas:21000},
tx:{to:'0x3333333333333333333333333333333333333333',chainId:46630,data:'0x12345678',value:'0x0'},
refund:{display:'999',bps:3},best:{of:1,refund_display:'999'},fee_sink:{label:'0.30% House pool fee'}},extra))});'''

def main():
    with sync_playwright() as p:
        browser=p.chromium.launch(channel='msedge',headless=True)
        page=browser.new_page()
        errors=[]
        page.on('pageerror',lambda error:errors.append(str(error)))
        page.set_content(HARNESS)
        page.evaluate('() => {' + MOCK + '}')
        page.add_script_tag(path=str(ROOT/'frontend/route.js'))
        page.evaluate("bootRoute({mode:'swap'})")
        page.evaluate("__setWallet({address:'0x4444444444444444444444444444444444444444',chain_id:46630,eth:'1',assets:[]})")
        page.locator('#amt').fill('1')
        page.wait_for_function('__requests.length===1')
        page.evaluate('respond(0)')
        page.wait_for_function('!document.getElementById("go").disabled')
        assert '999' not in page.locator('#route').inner_text()
        assert page.locator('#chip-refund').inner_text()=='None'
        # Clear input synchronously: the old executable quote must disappear immediately.
        page.locator('#amt').fill('')
        assert page.locator('#go').is_disabled()
        idle = page.locator('#route').inner_text()
        assert 'small $VAPURR' not in idle and '0.25%' not in idle
        page.locator('#amt').fill('2')
        page.wait_for_function('__requests.length===2')
        page.locator('#amt').fill('3')
        page.wait_for_function('__requests.length===3')
        page.evaluate('respond(2,{to_display:"6"})')
        page.wait_for_function('document.getElementById("to-amt").textContent==="6"')
        page.evaluate('respond(1,{to_display:"4"})')
        page.wait_for_timeout(40)
        assert page.locator('#to-amt').inner_text()=='6'
        # Change the amount while the review sheet is open, then accept the old review.
        page.locator('#go').click()
        page.locator('#amt').fill('4')
        page.evaluate('__approve(true)')
        page.wait_for_function('window.__failure')
        assert not page.evaluate('__messages.some(m=>m.cmd==="wallet-exec")')
        page.wait_for_function('__requests.length>=4')
        index=page.evaluate('__requests.length-1')
        page.evaluate('(i)=>respond(i)',index)
        page.wait_for_function('!document.getElementById("go").disabled')
        page.locator('#go').click()
        page.evaluate('__approve(true)')
        page.wait_for_function('__messages.some(m=>m.cmd==="wallet-exec")')
        sent=page.evaluate('__messages.find(m=>m.cmd==="wallet-exec")')
        assert sent['route_id']==f'route-{index}'
        assert sent['data']=='0x12345678'
        # Snapshots from another chain cannot finish this operation.
        page.evaluate("window.__failure=null; __setWallet({address:'0x4444444444444444444444444444444444444444',chain_id:46630,assets:[],tx:'other',tx_chain_id:1,tx_status:'confirmed'})")
        assert page.evaluate('window.__failure') is None
        assert not errors, errors
        browser.close()
    print('PASS: deployed-token picker, no phantom rebates, immediate invalidation, out-of-order replies, review edits, native route ID, cross-chain receipt isolation')

if __name__=='__main__': main()
