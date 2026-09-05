/* Navigation and diagrams only. Existing desk handlers own transactions. */
(function () {
  'use strict';
  var icons = {
    exchange:'M4 7h16m-4-4 4 4-4 4M20 17H4m4-4-4 4 4 4',
    save:'M4 9h16v11H4zM3 9l9-6 9 6M8 12v5m4-5v5m4-5v5',
    borrow:'M5 19V9h14v10H5ZM9 9V5h6v4m-6 5h6m-3-3v6',
    clock:'M12 7v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
    trade:'M4 17 9 12l4 3 7-10m-6 0h6v6M4 4v16h16',
    layers:'m3 7 9-4 9 4-9 4-9-4Zm0 5 9 4 9-4M3 17l9 4 9-4',
    bridge:'M3 19V5m18 14V5M3 8c6 7 12 7 18 0M3 17h18M8 12v5m8-5v5',
    nodes:'M9 5h6M7 8v8m10-8v8M9 19h6M3 3h6v5H3zM15 3h6v5h-6zM3 16h6v5H3zM15 16h6v5h-6z',
    chart:'M4 4v16h16M8 15v-4m5 4V7m5 8v-6',
    wallet:'M20 8V5H4v15h16V8H4m12 3h5v5h-5z',
    arrow:'M4 12h16m-6-6 6 6-6 6',
    lock:'M7 10V7a5 5 0 0 1 10 0v3M5 10h14v11H5zM12 14v3',
    back:'M15 6 9 12l6 6'
  };
  function icon(name) { return '<svg class="flow-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="'+(icons[name] || icons.exchange)+'"/></svg>'; }
  document.querySelectorAll('[data-icon]').forEach(function(el) { el.innerHTML=icon(el.dataset.icon); });

  var FINANCE_RE = /^vapurr:\/\/(defi|finance|lithe|oliver|euler|loop|house|lp|bonds|routing|markets|swap|bridge|pusd|mint|stake|wallet|portfolio)(\?|#|$)/i;
  var STACK_KEY = 'vapurr.financeStack';

  function pathToVapurr(path) {
    var raw = String(path || '');
    var m = raw.match(/\/([a-z0-9-]+)\.html/i);
    if (!m) return 'vapurr://defi';
    var id = m[1].toLowerCase();
    var rest = raw.slice(raw.indexOf('.html') + 5);
    if (id === 'pusd' || id === 'stake') {
      if (/oliver|euler|loop/i.test(rest)) return 'vapurr://oliver';
      if (/house/i.test(rest)) return 'vapurr://house';
      return 'vapurr://lithe';
    }
    if (id === 'defi') return 'vapurr://defi';
    return 'vapurr://' + id;
  }

  function readStack() {
    try { return JSON.parse(sessionStorage.getItem(STACK_KEY) || '[]'); } catch (e) { return []; }
  }
  function writeStack(stack) {
    try { sessionStorage.setItem(STACK_KEY, JSON.stringify(stack.slice(-24))); } catch (e) {}
  }
  function pushHere() {
    var here = (location.pathname || '') + (location.search || '') + (location.hash || '');
    var stack = readStack();
    if (stack[stack.length - 1] !== here) stack.push(here);
    writeStack(stack);
  }
  function financeBack() {
    var stack = readStack();
    while (stack.length) {
      var prev = stack.pop();
      writeStack(stack);
      if (!prev) continue;
      var target = pathToVapurr(prev);
      var cur = pathToVapurr((location.pathname||'')+(location.search||'')+(location.hash||''));
      if (target === cur) continue;
      if (window.vapurr) vapurr.go(target);
      return;
    }
    if (window.vapurr) vapurr.go('vapurr://defi');
  }
  function financeGo(target) {
    if (FINANCE_RE.test(target)) pushHere();
    if (window.vapurr) vapurr.go(target);
  }

  var nav=document.querySelector('[data-finance-nav]');
  if(nav) {
    var desk=document.body.dataset.desk;
    if(desk==='cash' && /oliver|euler|loop/i.test(location.hash+location.search)) desk='credit';
    if(desk==='cash' && /house/i.test(location.hash+location.search)) desk='house';
    var routeDesk = desk==='swap' || desk==='bridge';
    var backBtn = routeDesk
      ? '<button type="button" class="finance-back" data-finance-back aria-label="Back">'+icon('back')+'<span>Back</span></button>'
      : '';
    nav.innerHTML=backBtn+'<nav class="finance-nav" aria-label="DeFi"><button class="finance-brand" data-go="vapurr://defi" aria-label="VAPURR DeFi home"><img src="/cat.svg" alt=""/>VAPURR</button><div class="finance-nav-links">'+[
      ['overview','Overview','vapurr://defi'],['cash','Lithe','vapurr://lithe'],['credit','Oliver','vapurr://oliver'],['savings','Save & bond','vapurr://bonds'],['house','House','vapurr://house'],['swap','Swap','vapurr://swap'],['bridge','Bridge','vapurr://bridge']
    ].map(function(item){ return '<button data-go="'+item[2]+'"'+(item[0]===desk?' aria-current="page"':'')+'>'+item[1]+'</button>'; }).join('')+'</div><button class="finance-wallet" data-go="vapurr://wallet">'+icon('wallet')+'Wallet</button></nav>';
  }

  document.querySelectorAll('[data-finance-back]').forEach(function(el) {
    el.addEventListener('click', function(){ financeBack(); });
  });

  document.querySelectorAll('[data-go]').forEach(function(el) {
    if(!el.onclick) el.addEventListener('click',function(){
      var target=el.dataset.go;
      if(target.charAt(0)==='#') { var node=document.querySelector(target); if(node) node.scrollIntoView({behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth'}); }
      else if(document.body.dataset.desk==='cash' && /^vapurr:\/\/(lithe|oliver|house)$/.test(target)) {
        var panel={lithe:'book',oliver:'euler',house:'house'}[target.split('//')[1]];
        selectDesk(panel, true);
      }
      else if(FINANCE_RE.test(target)) financeGo(target);
      else if(window.vapurr) vapurr.go(target);
    });
  });
  function selectDesk(panel, focus) {
    document.body.dataset.activeDesk=panel;
    document.querySelectorAll('[data-jump]').forEach(function(tab){
      var selected=tab.dataset.jump===panel;
      tab.setAttribute('aria-selected',String(selected)); tab.tabIndex=selected?0:-1;
    });
    var deskName={book:'lithe',euler:'oliver',house:'house'}[panel];
    document.querySelectorAll('.finance-nav-links [data-go]').forEach(function(button){
      if(button.dataset.go==='vapurr://'+deskName) button.setAttribute('aria-current','page');
      else button.removeAttribute('aria-current');
    });
    if(focus) {
      history.replaceState(null,'','#'+deskName);
      var chrome=document.querySelector('[data-finance-nav]');
      if(chrome) chrome.scrollIntoView({block:'start'});
      else window.scrollTo(0,0);
    }
  }
  if(document.body.dataset.desk==='cash') {
    var requested=location.hash+location.search;
    selectDesk(/oliver|euler|loop/i.test(requested)?'euler':/house/i.test(requested)?'house':'book',false);
    document.querySelectorAll('[data-jump]').forEach(function(tab){
      tab.addEventListener('click',function(){selectDesk(tab.dataset.jump,false);});
      tab.addEventListener('keydown',function(event){
        var tabs=Array.from(document.querySelectorAll('[data-jump]')), i=tabs.indexOf(tab), next;
        if(event.key==='ArrowRight') next=tabs[(i+1)%tabs.length]; if(event.key==='ArrowLeft') next=tabs[(i+tabs.length-1)%tabs.length];
        if(event.key==='Home') next=tabs[0]; if(event.key==='End') next=tabs[tabs.length-1];
        if(next){event.preventDefault();next.click();next.focus();}
      });
    });
    document.querySelectorAll('[data-mode]').forEach(function(button){button.addEventListener('click',function(){selectDesk('book',false);});});
  }
  var flows={
    mint:['01','CASH · LITHE','VAPURR<br/>meets PUSD.','Mint PUSD. Redeem back to VAPURR.','V','VAPURR','You exchange','Lithe','Mint ↔ redeem','P','PUSD','You receive','At exchange','Oracle + spread','Open Lithe','vapurr://lithe','exchange'],
    supply:['02','CREDIT · OLIVER','Put PUSD<br/>to work.','Supply the lending pool. Withdraw from available cash.','P','PUSD','You supply','Oliver','Supply → interest','P','Position','You hold','Yield','Variable','Open Oliver','vapurr://oliver','save'],
    borrow:['03','CREDIT · OLIVER','Access cash.<br/>Keep exposure.','Borrow against collateral. Track debt and liquidation risk.','V','Collateral','You deposit','Oliver','Collateral → credit','P','PUSD','You borrow','Position','Interest + LTV','Open Oliver','vapurr://oliver','borrow'],
    bond:['04','CAPITAL · BONDS','Assets in.<br/>Equity ahead.','Bond an asset. Claim gV after vesting.','↗','Asset','You bond','Bonds','Deposit → vest','gV','gV','You claim','Terms','Discount + vesting','Open bonds','vapurr://bonds','clock'],
    trade:['05','MARKETS · HOUSE','Equity<br/>meets cash.','Exchange wrapped gV and PUSD through House.','gV','wgV','You exchange','House','wgV ↔ PUSD','P','PUSD','You receive','At exchange','Price + fee','Open House','vapurr://house','trade']
  };
  document.querySelectorAll('[data-flow]').forEach(function(button){button.addEventListener('click',function(){
    var f=flows[button.dataset.flow];
    document.querySelectorAll('[data-flow]').forEach(function(b){b.setAttribute('aria-pressed',String(b===button));});
    ['flow-number','flow-label','flow-title','flow-description','flow-in-icon','flow-in','flow-in-note','flow-engine','flow-engine-note','flow-out-icon','flow-out','flow-out-note','flow-term-key','flow-term-value'].forEach(function(id,i){
      var el=document.getElementById(id); if(i===2) el.innerHTML=f[i]; else el.textContent=i===0?f[i]+' / 05':f[i];
    });
    var cta=document.getElementById('flow-open'); cta.innerHTML=f[14]+' <span aria-hidden="true">↗</span>'; cta.dataset.go=f[15];
    document.getElementById('flow-icon').innerHTML=icon(f[16]);
    document.getElementById('route-art').setAttribute('aria-label',f[5]+' through '+f[7]+' to '+f[10]);
  });});
  var ltv=document.getElementById('e-ltv'), max=document.getElementById('e-maxltv'), meter=document.getElementById('credit-meter');
  if(ltv && max && meter) {
    function paintMeter(){
      var value=parseFloat(ltv.textContent), limit=parseFloat(max.textContent);
      var valid=Number.isFinite(value)&&Number.isFinite(limit)&&limit>0;
      document.getElementById('credit-meter-label').textContent=valid?ltv.textContent+' / '+max.textContent+' max':'Waiting for position';
      meter.querySelector('.meter-fill').style.width=(valid?Math.max(0,Math.min(100,value/limit*100)):0)+'%';
      meter.setAttribute('aria-label',valid?'Loan to value '+ltv.textContent+', maximum '+max.textContent:'Loan to value unavailable');
    }
    new MutationObserver(paintMeter).observe(ltv,{childList:true,subtree:true,characterData:true});
    new MutationObserver(paintMeter).observe(max,{childList:true,subtree:true,characterData:true}); paintMeter();
  }
  document.querySelectorAll('[data-asset]').forEach(function(tab){tab.addEventListener('keydown',function(event){
    var tabs=Array.from(document.querySelectorAll('[data-asset]')), i=tabs.indexOf(tab), next;
    if(event.key==='ArrowRight') next=tabs[(i+1)%tabs.length]; if(event.key==='ArrowLeft') next=tabs[(i+tabs.length-1)%tabs.length];
    if(event.key==='Home') next=tabs[0]; if(event.key==='End') next=tabs[tabs.length-1];
    if(next){event.preventDefault();next.focus();next.click();}
  });});
})();
