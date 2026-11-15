import {describe, it, expect} from 'vitest';
import {time, lyricView, readingStatus} from './view';
import type {Snapshot} from './types';
const base = {track: {title:'Own',artist:'Author',album:'',duration_ms:10000}, selected_session:'1', lyrics:null, lyrics_status:'missing',active_line:null,timing_reliable:true} as Snapshot;
describe('presentation', () => {
  it('distinguishes untimed text from unconfirmed versions and missing playback position',()=>{
    const plain:NonNullable<Snapshot['lyrics']>={format:'plain',lines:[],plain_text:'Texto propio',source:'Prueba',source_id:null,warnings:[]};
    const synced={...plain,format:'line_synced',lines:[{at_ms:0,text:'Texto propio'}]} as NonNullable<Snapshot['lyrics']>;
    expect(readingStatus({...base,sync_confirmed:false},plain)?.title).toBe('Modo lectura · sin tiempos');
    expect(readingStatus({...base,sync_confirmed:false},synced)?.title).toContain('versión por confirmar');
    expect(readingStatus({...base,timing_reliable:false},synced)?.title).toContain('sin posición fiable');
    expect(readingStatus({...base,settings:{karaoke_mode:'word'} as Snapshot['settings']},synced)).toBeNull();
  });
  it('formats progress with tabular seconds', () => { expect(time(-10)).toBe('0:00'); expect(time(65000)).toBe('1:05'); });
  it('distinguishes no session, missing lyrics, loading, and errors', () => {
    expect(lyricView({...base,track:null,selected_session:null}).current).toContain('reproductor');
    expect(lyricView(base).current).toBe('Aún no hay letra');
    expect(lyricView({...base,lyrics_status:'loading'}).current).toContain('Buscando');
    expect(lyricView({...base,lyrics_status:'error'}).current).toContain('No se pudo');
  });
  it('shows neighbors only from the active document and handles unavailable timing', () => {
    const state = {...base,active_line:1,lyrics:{format:'line_synced',lines:[{at_ms:0,text:'Before'},{at_ms:1000,text:'Now'},{at_ms:2000,text:'After'}]}} as Snapshot;
    expect(lyricView(state)).toEqual({previous:'Before',current:'Now',next:'After'});
    expect(lyricView({...state,timing_reliable:false}).current).toContain('no disponible');
  });
});
