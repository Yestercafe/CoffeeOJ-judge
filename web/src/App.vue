<script lang="ts">
import axios from 'axios'

export default {
  data() {
    return {
      source: '',
      lang: 'cpp',
      problem_id: '',
      is_submitted: false,
      info: 'Judging ...',
    }
  },
  methods: {
    async submit() {
      this.is_submitted = true
      const data = {
        source: this.source,
        lang: this.lang,
        problem_id: this.problem_id,
      }
      const headers = {
        'Content-Type': 'application/json'
      }
      try {
        this.info = 'Judging ...'
        const resp = await axios.post('/api/v1/submit', data, { headers: headers })
        this.info = resp.data
        console.log(resp)
      } catch (e) {
        console.log(e)
      }
    }
  }
}
</script>

<template>
  <header>
  </header>
  <main>
    <h1>CoffeeOJ Judge</h1>
    <div style="display: block;">
      <div class="item">
        <label for="problem_id">Problem ID: </label>
        <input type="text" name="problem_id" placeholder="problem ID" v-model="problem_id" required>
      </div>
      <div class="item">
        <label for="lang">Language: </label>
        <select name="lang" v-model="lang">
          <option value="c">C</option>
          <option value="cpp" selected>C++</option>
          <option value="rust">Rust</option>
          <option value="python">Python</option>
        </select>
      </div>
      <div class="item">
        <textarea rows="25" cols="80" name="source" placeholder="input your code" v-model="source" required />
      </div>
      <button id="submit" @click="submit">submit</button>
      <div class="output" v-if="is_submitted">
        <h2>{{ info }}</h2>
      </div>
    </div>
  </main>
</template>

<style>
</style>
