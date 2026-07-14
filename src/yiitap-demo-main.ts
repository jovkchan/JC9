import { createApp } from 'vue'
import YiitapDemo from '@/components/YiitapDemo.vue'
import '@/styles/global.scss'
import '@/styles/yiitap-adapter.scss'
import '@yiitap/vue/dist/vue.css'

const app = createApp(YiitapDemo)
app.mount('#app')
