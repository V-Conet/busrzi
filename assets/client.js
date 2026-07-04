(function () {
  'use strict';

  function ready(callback) {
    if (document.readyState !== 'loading') {
      callback();
    } else {
      document.addEventListener('DOMContentLoaded', callback, { once: true });
    }
  }

  function getCookie(name) {
    const match = document.cookie.match(
      new RegExp('(?:^|; )' + name.replace(/([.$?*|{}()[\]\\/+^])/g, '\\$1') + '=([^;]*)')
    );
    return match ? decodeURIComponent(match[1]) : null;
  }

  function setCookie(name, value, maxAge) {
    document.cookie = name + '=' + value + '; path=/; max-age=' + maxAge + '; samesite=lax';
  }

  function updateCounters(data) {
    var mapping = {
      busrzi_page_pv: 'page_pv',
      busrzi_site_pv: 'site_pv',
      busrzi_site_uv: 'site_uv',
    };

    for (var id in mapping) {
      if (!Object.prototype.hasOwnProperty.call(mapping, id)) continue;
      var el = document.getElementById(id);
      if (el) {
        var value = data && data[mapping[id]];
        el.textContent = String(value === undefined || value === null ? 0 : value);
      }
    }
  }

  function init() {
    var script = document.currentScript;
    var apiUrl = script ? new URL('/api/counter', script.src).href : '/api/counter';

    var host = location.host || 'unknown-host';
    var cookieName = 'busrzi_uv_' + host.replace(/[^a-zA-Z0-9_-]/g, '_');
    var isNewUv = !getCookie(cookieName);

    if (isNewUv) {
      setCookie(cookieName, '1', 60 * 60 * 24 * 365);
    }

    fetch(apiUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: location.href, is_new_uv: isNewUv }),
    })
      .then(function (res) {
        if (!res.ok) throw new Error('HTTP ' + res.status);
        return res.json();
      })
      .then(function (json) {
        if (json && json.ok === false) {
          console.warn('[busrzi] api error:', json.error);
          return;
        }
        updateCounters(json && json.result ? json.result : json);
      })
      .catch(function (err) {
        console.warn('[busrzi] failed to fetch counter:', err);
      });
  }

  ready(init);
})();
