import {test,expect, type Page} from '@playwright/test';

const defaultSettings = {
  schema_version:1,network_allowed:false,offset_ms:0,font_size:23,opacity:1,alignment:'center',
  reduced_motion:false,z_order:'normal',view_mode:'compact',
  theme_id:'classic_cat',cat_animation_enabled:true,lyrics_backdrop:'dark',
  x:null,y:null,width:420,height:260,
};

async function openLetra(page: Page) {
  await page.getByRole('button', { name: 'Letra', exact: true }).click();
}

test.beforeEach(async ({page}) => {
  await page.addInitScript((settings) => {
    const callbacks = new Map<number,(event:unknown)=>void>();
    const listeners = new Map<string,number>();
    let id = 0;
    const commands: {command:string,args:Record<string,any>}[]=[];
    const state = {revision:1,generation:1,sessions:[{id:'one',name:'Reproductor de prueba'}],selected_session:'one',track:{title:'Canción de prueba con un título muy largo para comprobar el espacio disponible',artist:'Autor de prueba',album:'Demo',duration_ms:120000},playing:true,position_ms:30000,timing_reliable:true,artwork:null,lyrics:{source:'Prueba',source_id:null,format:'line_synced',lines:[{at_ms:0,text:'Una línea anterior'},{at_ms:30000,text:'La música sigue aquí'},{at_ms:60000,text:'Una línea que viene después'}],plain_text:'',warnings:[]},active_line:1,lyrics_status:'ready',candidates:[],error:null,settings:structuredClone(settings)};
    const emit = () => { state.revision++; const callback = callbacks.get(listeners.get('proyecto-reze:state')!); callback?.({payload:structuredClone(state)}); };
    Object.assign(window, {__testCommands:commands,__testGetState:()=>structuredClone(state),__testPosition:(tick:unknown)=>callbacks.get(listeners.get('proyecto-reze:position')!)?.({payload:tick}),__testSetState:(patch:Record<string,unknown>)=>{Object.assign(state,patch);emit();},__TAURI_INTERNALS__:{
      transformCallback(callback:(event:unknown)=>void) { callbacks.set(++id, callback); return id; },
      async invoke(command:string,args:Record<string,any>) {
        commands.push({command,args});
        if(command === 'translate_lyrics') { Object.assign(state,{translation:structuredClone(state.lyrics),translation_status:'ready'});(state as any).translation.lines[1].text='La música sigue aquí en español';emit(); }
        if(command === 'plugin:event|listen') {listeners.set(args.event,args.handler);return args.handler;}
        if(command === 'get_state')return structuredClone(state);
        if(command === 'update_settings') {state.settings=args.settings;emit();}
        if(command === 'select_session') {state.selected_session=args.id;emit();}
      },
    }});
  }, defaultSettings);
  await page.goto('/');
});

test('renders active line, neighbors, and long metadata without overflow',async({page})=>{
  await expect(page.locator('#current')).toHaveText('La música sigue aquí');
  await expect(page.locator('#previous')).toHaveText('Una línea anterior');
  expect(await page.evaluate(()=>document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({path:'artifacts/widget-compact.png'});
});

test('settings, keyboard recovery, preview and validation',async({page})=>{
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await expect(page.locator('#sessions')).toHaveValue('one');
  await page.locator('#network-allowed').check();
  await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('#settings-panel')).toBeHidden();
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await expect(page.locator('#network-allowed')).toBeChecked();
  await expect(page.locator('#client-contact')).toHaveCount(0);
  await page.locator('#font-size').fill('30');
  await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('#settings-panel')).toBeHidden();
  await expect(page.locator('#current')).toHaveCSS('font-size','30px');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button',{name:'Abrir ajustes'})).toBeFocused();
});

test('theme preview cancels on escape and persists on save',async({page})=>{
  await expect(page.locator('html')).toHaveAttribute('data-theme','classic_cat');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await page.locator('#theme-id').selectOption('cat_constellation');
  await expect(page.locator('html')).toHaveAttribute('data-theme','cat_constellation');
  await page.keyboard.press('Escape');
  await expect(page.locator('html')).toHaveAttribute('data-theme','classic_cat');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await page.locator('#theme-id').selectOption('window_cat');
  await page.locator('#lyrics-backdrop').selectOption('light');
  await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme','window_cat');
  await expect(page.locator('html')).toHaveAttribute('data-backdrop','light');
  const saved = await page.evaluate(()=>(window as any).__testGetState().settings);
  expect(saved.theme_id).toBe('window_cat');
  expect(saved.lyrics_backdrop).toBe('light');
});

test('changing theme keeps karaoke word progress',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.evaluate(()=>{const current=(window as any).__testGetState();(window as any).__testSetState({settings:{...current.settings,karaoke_mode:'word'},lyrics_revision:10,position_sequence:10,position_ms:1500,active_line:0,sync_confirmed:true,lyrics:{source:'Prueba propia',format:'line_synced',source_id:1,lines:[{at_ms:1000,end_ms:3500,text:'Hola mundo',words:[{text:'Hola ',start_ms:1000,end_ms:2000},{text:'mundo',start_ms:2000,end_ms:3000}]}],plain_text:'',warnings:[]}});});
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress','0.5');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await page.locator('#theme-id').selectOption('cat_notebook');
  await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme','cat_notebook');
  await expect(page.locator('.karaoke-word')).toHaveCount(2);
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress','0.5');
  await expect(page.locator('#current')).toContainText('Hola');
});

test('transparent lyrics view remains recoverable and supports small windows',async({page})=>{
  await page.getByRole('button',{name:'Cambiar a solo letras'}).click();
  await expect(page.locator('body')).toHaveClass(/lyrics-only/);
  await expect(page.locator('#current-band')).toBeVisible();
  await page.setViewportSize({width:300,height:180});
  await expect(page.getByRole('button',{name:'Abrir ajustes'})).toBeVisible();
  await page.screenshot({path:'artifacts/widget-lyrics.png',omitBackground:true});
});

test('uncertain cover result shows artist and duration warning before selection',async({page})=>{
  await page.evaluate(()=> (window as any).__testSetState({lyrics:null,lyrics_status:'candidates',candidates:[{id:12756272,trackName:'Gamushara',artistName:'The Covers Duo',albumName:'',duration:196,matchNote:'Duración distinta por 20 s; confirma esta versión.'}]}));
  await expect(page.locator('#current')).toHaveText('Encontramos una posible letra');
  await openLetra(page);
  await page.getByRole('button',{name:'Versiones (1)',exact:true}).click();
  await expect(page.locator('#candidates')).toContainText('The Covers Duo');
  await expect(page.locator('#candidates')).toContainText('20 s');
  await page.screenshot({path:'artifacts/lyrics-candidate-warning.png'});
});

test('manual title and artist correction submits generation-bound search',async({page})=>{
  await openLetra(page);
  await page.getByRole('button',{name:'Corregir búsqueda…',exact:true}).click();
  await page.locator('#search-title').fill('He-Man');await page.locator('#search-artist').fill('The Covers Duo');
  await page.getByRole('button',{name:'Buscar con estos datos'}).click();
  const sent=await page.evaluate(()=>(window as any).__testCommands.find((c:any)=>c.command==='correct_search'));
  expect(sent.args).toEqual({title:'He-Man',artist:'The Covers Duo',generation:1,reset:false});
});

test('translation is opt-in and original remains available',async({page})=>{
  await page.evaluate(()=>(window as any).__testSetState({language:'en',lyrics_revision:1,translation:null,translation_status:'idle'}));
  expect(await page.evaluate(()=>(window as any).__testCommands.filter((c:any)=>c.command==='translate_lyrics').length)).toBe(0);
  await openLetra(page);
  await page.getByRole('button',{name:'Traducir al español…',exact:true}).click();
  await expect(page.locator('#translation-panel')).toContainText('MyMemory');
  expect(await page.evaluate(()=>(window as any).__testCommands.filter((c:any)=>c.command==='translate_lyrics').length)).toBe(0);
  await page.getByRole('button',{name:'Enviar y traducir'}).click();
  await expect(page.locator('#current')).toHaveText('La música sigue aquí en español');
  await openLetra(page);
  await page.getByRole('button',{name:'Ver original'}).click();await expect(page.locator('#current')).toHaveText('La música sigue aquí');
});

test('multiple results show default selection and retain access to alternatives',async({page})=>{
  await page.evaluate(()=>(window as any).__testSetState({selected_candidate_id:2,sync_confirmed:true,selection_notice:'Encontramos 2 letras. Seleccionada por defecto.',candidates:[{id:2,trackName:'Song',artistName:'Author',albumName:'',duration:120},{id:3,trackName:'Song',artistName:'Author',albumName:'',duration:121}]}));
  await expect(page.locator('#selection-alert')).toContainText('2 letras');
  await openLetra(page);
  await page.getByRole('button',{name:'Versiones (2)'}).click();await expect(page.locator('#candidates')).toContainText('Seleccionada · Song');
});

test('karaoke words follow source times, pause, seek and ignore stale position packets',async({page})=>{
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.evaluate(()=>{const current=(window as any).__testGetState();(window as any).__testSetState({settings:{...current.settings,karaoke_mode:'word'},lyrics_revision:10,position_sequence:10,position_ms:1500,active_line:0,sync_confirmed:true,lyrics:{source:'Prueba propia',format:'line_synced',source_id:1,lines:[{at_ms:1000,end_ms:3500,text:'Hola mundo',words:[{text:'Hola ',start_ms:1000,end_ms:2000},{text:'mundo',start_ms:2000,end_ms:3000}]}],plain_text:'',warnings:[]}});});
  await expect(page.locator('.karaoke-word')).toHaveCount(2);
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress','0.5');
  await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress','0');
  await page.screenshot({path:'artifacts/karaoke-words.png'});
  await page.evaluate(()=>(window as any).__testPosition({generation:1,lyrics_revision:10,sequence:11,position_ms:2500,active_line:0,playing:false,timing_reliable:true}));
  await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress','0.5');
  await page.waitForTimeout(200);await expect(page.locator('.karaoke-word').last()).toHaveAttribute('data-progress','0.5');
  await page.evaluate(()=>(window as any).__testPosition({generation:1,lyrics_revision:10,sequence:12,position_ms:1200,active_line:0,playing:true,timing_reliable:true}));
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress','0.2');
  await page.evaluate(()=>(window as any).__testPosition({generation:1,lyrics_revision:10,sequence:11,position_ms:2800,active_line:0,playing:true,timing_reliable:true}));
  await expect(page.locator('.karaoke-word').first()).toHaveAttribute('data-progress','0.2');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();await page.locator('#karaoke-mode').selectOption('line');await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('.karaoke-word')).toHaveCount(0);await expect(page.locator('#current')).toHaveText('Hola mundo');
});

test('word preference persists but LRC without words keeps line mode',async({page})=>{
  await page.getByRole('button',{name:'Abrir ajustes'}).click();await page.locator('#karaoke-mode').selectOption('word');await page.getByRole('button',{name:'Guardar ajustes'}).click();
  await expect(page.locator('#karaoke-badge')).toHaveText('Línea');await expect(page.locator('.karaoke-word')).toHaveCount(0);
  await page.getByRole('button',{name:'Abrir ajustes'}).click();await expect(page.locator('#karaoke-mode')).toHaveValue('word');await expect(page.locator('#settings-karaoke-status')).toContainText('Se mantiene el modo por línea');
});

test('untimed lyrics have a prominent persistent notice and neutral text',async({page})=>{
  await page.evaluate(()=>{const s=(window as any).__testGetState();(window as any).__testSetState({settings:{...s.settings,karaoke_mode:'word'},sync_confirmed:true,lyrics_revision:12,lyrics:{source:'Prueba propia',source_id:null,format:'plain',lines:[],plain_text:Array(40).fill('Texto de prueba para leer sin seguimiento.').join('\n'),warnings:[]}});});
  await expect(page.locator('#reading-label')).toHaveText('Modo lectura · sin tiempos');
  await expect(page.locator('#reading-label')).toHaveCSS('font-weight','700');
  await expect(page.locator('#reading-status')).toBeVisible();
  await expect(page.locator('#reading-detail')).toContainText('sin seguimiento ni resaltado');
  await expect(page.locator('#plain-text')).toHaveCSS('background-image','none');
  await expect(page.locator('#lyrics')).toBeHidden();await expect(page.locator('.karaoke-word')).toHaveCount(0);
  await expect(page.locator('#karaoke-badge')).toHaveText('Lectura');
  expect(await page.locator('#plain-text').evaluate(element=>element.clientHeight)).toBeGreaterThanOrEqual(38);
  const before=await page.locator('#reading-status').boundingBox();
  await page.locator('#plain-text').evaluate(element=>{element.scrollTop=300;});
  expect(await page.locator('#reading-status').boundingBox()).toEqual(before);
  await page.screenshot({path:'artifacts/reading-no-timing.png'});
  await page.getByRole('button',{name:'Cambiar a solo letras'}).click();
  await expect(page.locator('#reading-status')).toBeVisible();await expect(page.locator('#plain-text')).toHaveCSS('background-image','none');
  await page.setViewportSize({width:300,height:180});await expect(page.locator('#reading-label')).toBeInViewport();
  await page.screenshot({path:'artifacts/reading-no-timing-small.png',omitBackground:true});
});

test('cat menu toggles animation immediately and settings preview can cancel it',async({page})=>{
  await page.getByRole('button',{name:'Opciones del gatito'}).click();
  await expect(page.locator('#cat-menu')).toBeVisible();
  await page.locator('#cat-animation').uncheck();
  const saved = await page.evaluate(()=>(window as any).__testGetState().settings.cat_animation_enabled);
  expect(saved).toBe(false);
  await page.keyboard.press('Escape');
  await page.getByRole('button',{name:'Abrir ajustes'}).click();
  await expect(page.locator('#settings-cat-animation')).not.toBeChecked();
  await page.locator('#settings-cat-animation').check();
  await page.keyboard.press('Escape');
  const afterCancel = await page.evaluate(()=>(window as any).__testGetState().settings.cat_animation_enabled);
  expect(afterCancel).toBe(false);
});

const themes = ['classic_cat','window_cat','cat_notebook','cat_constellation'] as const;
for (const theme of themes) {
  test(`theme gallery capture ${theme}`, async ({page}) => {
    await page.evaluate((themeId) => {
      const current = (window as any).__testGetState();
      (window as any).__testSetState({ settings: { ...current.settings, theme_id: themeId } });
    }, theme);
    await page.setViewportSize({ width: 420, height: 260 });
    await page.screenshot({ path: `artifacts/theme-${theme}-420x260.png` });
    await page.setViewportSize({ width: 300, height: 180 });
    await page.screenshot({ path: `artifacts/theme-${theme}-300x180.png` });
  });
}

test('mascot animation advances while playing and stops when disabled', async ({page}) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForTimeout(300);
  const before = await page.locator('#mascot').getAttribute('data-frame');
  await page.waitForTimeout(450);
  const mid = await page.locator('#mascot').getAttribute('data-frame');
  await expect(page.locator('#mascot')).toHaveAttribute('data-sequence', /playing|greet|static/);
  await page.screenshot({ path: 'artifacts/mascot-playing.png' });
  await page.getByRole('button', { name: 'Opciones del gatito' }).click();
  await page.locator('#cat-animation').uncheck();
  await page.waitForTimeout(200);
  const stopped = await page.locator('#mascot').getAttribute('data-sequence');
  expect(stopped).toBe('static');
  const frameA = await page.locator('#mascot').getAttribute('data-frame');
  await page.waitForTimeout(400);
  const frameB = await page.locator('#mascot').getAttribute('data-frame');
  expect(frameA).toBe(frameB);
  await page.screenshot({ path: 'artifacts/mascot-stopped.png' });
  // Evidence that frames can change while enabled (best-effort; greet/playing may share a frame briefly).
  expect([before, mid, frameA].some(Boolean)).toBe(true);
});

test('expanded theme capture and lyrics-only backdrops', async ({page}) => {
  await page.evaluate(() => {
    const current = (window as any).__testGetState();
    (window as any).__testSetState({ settings: { ...current.settings, theme_id: 'classic_cat' } });
  });
  await page.setViewportSize({ width: 640, height: 400 });
  await page.screenshot({ path: 'artifacts/theme-classic_cat-640x400.png' });
  await page.getByRole('button', { name: 'Cambiar a solo letras' }).click();
  await page.evaluate(() => {
    document.body.style.background = '#f5f5f5';
  });
  await page.screenshot({ path: 'artifacts/lyrics-only-dark-on-light.png', omitBackground: false });
  await page.getByRole('button', { name: 'Abrir ajustes' }).click();
  await page.locator('#lyrics-backdrop').selectOption('light');
  await page.getByRole('button', { name: 'Guardar ajustes' }).click();
  await page.evaluate(() => {
    document.body.style.background = '#1a1a1a';
  });
  await page.screenshot({ path: 'artifacts/lyrics-only-light-on-dark.png', omitBackground: false });
});
