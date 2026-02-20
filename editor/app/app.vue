<template>
  <div class="h-screen">
    <NuxtRouteAnnouncer />
    <NuxtPage />
  </div>
</template>
<style lang="css">
html,
body,
#__nuxt,
#__layout,
* {
  scrollbar-width: thin;
  scrollbar-color: #4b5563 #111827;
}

*::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}

*::-webkit-scrollbar-track {
  background: #111827;
}

*::-webkit-scrollbar-thumb {
  background-color: #4b5563;
  border-radius: 8px;
  border: 2px solid #111827;
}

*::-webkit-scrollbar-thumb:hover {
  background-color: #6b7280;
}

</style>