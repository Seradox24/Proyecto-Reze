import {describe,it,expect} from 'vitest';
import {wordProgress,canHighlightWords,modeStatus} from './karaoke';
import type {Snapshot} from './types';
describe('word timing',()=>{
  it('uses source boundaries and resets after seeking backwards',()=>{
    const word={text:'Hola ',start_ms:1000,end_ms:2000};
    expect([500,1000,1500,2000,3000,1200].map(p=>wordProgress(word,p))).toEqual([0,0,.5,1,1,.2]);
    expect(wordProgress(word,1500)).toBe(wordProgress(word,1500));
  });
  it('never invents missing end times and honors reduced motion',()=>{
    expect(wordProgress({text:'A',start_ms:1000},999)).toBe(0);
    expect(wordProgress({text:'A',start_ms:1000},1000)).toBe(1);
    expect(wordProgress({text:'A',start_ms:1000,end_ms:2000},1200,true)).toBe(1);
  });
  it('falls back for missing words, translations, unconfirmed versions and unreliable clocks',()=>{
    const lyrics={format:'line_synced',lines:[{at_ms:0,text:'Hola',words:[{text:'Hola',start_ms:0,end_ms:1000}]}]} as NonNullable<Snapshot['lyrics']>;
    const state={settings:{karaoke_mode:'word'},active_line:0,sync_confirmed:true,timing_reliable:true} as Snapshot;
    expect(canHighlightWords(state,lyrics)).toBe(true);expect(canHighlightWords(state,lyrics,true)).toBe(false);
    expect(canHighlightWords({...state,sync_confirmed:false},lyrics)).toBe(false);expect(canHighlightWords({...state,timing_reliable:false},lyrics)).toBe(false);
    const plain={...lyrics,lines:[{at_ms:0,text:'Hola'}]};expect(canHighlightWords(state,plain)).toBe(false);expect(modeStatus(state,plain)).toContain('Se mantiene el modo por línea');
  });
});
