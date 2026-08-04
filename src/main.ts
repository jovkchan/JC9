import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { YiiEditorPlugin } from '@yiitap/vue'
import App from './App.vue'
import { useEffectConfig } from './stores/effectConfig'
import './styles/global.scss'
import './components/ui/tokens.scss'
import '@yiitap/vue/dist/vue.css'
import './styles/yiitap-adapter.scss'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
// 初始化全局动画效果配置（流光/光晕：CSS 变量 + html class + DATA JSON 持久化）
useEffectConfig()
app.use(YiiEditorPlugin)
app.mount('#app')
