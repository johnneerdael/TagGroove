import test from 'node:test';
import assert from 'node:assert/strict';
import {Window} from 'happy-dom';
import {loadProductionModule} from './helpers/load-production-module.mjs';
const {bindRowDrag}=await loadProductionModule('list-behaviour.ts');
const deferred=()=>{let resolve;return{promise:new Promise(done=>resolve=done),resolve};};
const tick=()=>new Promise(resolve=>setTimeout(resolve,0));

test('dragging an unselected row waits for guarded selection and never moves the old selection',async()=>{
  for(const outcome of ['accept','replace','refuse','release','cancel']){
    const window=new Window(),gate=deferred();try{
      const document=window.document;document.body.innerHTML='<div id="wrap"><table><tbody><tr data-path="a"><td>a</td></tr><tr data-path="b"><td>b</td></tr></tbody></table></div>';
      const wrap=document.querySelector('#wrap'),selected=new Set(['a']),drops=[];
      bindRowDrag(wrap,{currentWrap:()=>document.querySelector('#wrap'),rows:()=>['a','b'],rowHeight:()=>24,headerHeight:20,selected:()=>selected,
        select:async path=>{await gate.promise;if(outcome==='refuse')return false;selected.clear();selected.add(path);if(outcome==='replace')document.body.innerHTML='<div id="wrap"><table><tbody><tr data-path="a"></tr><tr data-path="b"></tr></tbody></table></div>';return true;},
        drop:(moving,before)=>drops.push({moving,before})});
      const pointer=(type,target,y)=>target.dispatchEvent(new window.PointerEvent(type,{bubbles:true,button:0,altKey:true,pointerId:1,clientY:y}));
      pointer('pointerdown',document.querySelector('[data-path="b"]'),50);
      pointer('pointermove',wrap,80);
      assert.equal(!!wrap.querySelector('.row-drop-marker'),false,'no drag plan before selection is accepted');
      if(outcome==='release'||outcome==='cancel')pointer(outcome==='release'?'pointerup':'pointercancel',wrap,80);
      gate.resolve();await tick();
      if(outcome==='accept'||outcome==='replace'){
        const current=document.querySelector('#wrap');assert.equal(!!current.querySelector('.row-drop-marker'),true,outcome);pointer('pointerup',current,80);
        assert.deepEqual(drops,[{moving:['b'],before:null}]);
      }else{
        pointer('pointerup',wrap,80);assert.deepEqual(drops,[]);assert.equal(wrap.querySelector('.row-drop-marker'),null);
      }
    }finally{gate.resolve();await tick();window.happyDOM.abort();}
  }
});
