import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { YiiEditorPlugin } from '@yiitap/vue'
import App from './App.vue'
import './styles/global.scss'
import '@yiitap/vue/dist/vue.css'
import './styles/yiitap-adapter.scss'

const app = createApp(App)
app.use(createPinia())
app.use(YiiEditorPlugin)
app.mount('#app')
