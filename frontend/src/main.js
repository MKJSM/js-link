import './styles/main.css';

// FontAwesome
import '@fortawesome/fontawesome-free/css/all.css';

// CodeMirror
import CodeMirror from 'codemirror';
import 'codemirror/lib/codemirror.css';
import 'codemirror/theme/dracula.css';
import 'codemirror/addon/hint/show-hint.css';
import 'codemirror/addon/lint/lint.css';

import 'codemirror/mode/javascript/javascript';
import 'codemirror/addon/edit/closebrackets';
import 'codemirror/addon/edit/matchbrackets';
import 'codemirror/addon/hint/show-hint';
import 'codemirror/addon/hint/javascript-hint';
import 'codemirror/addon/lint/lint';
import 'codemirror/addon/lint/json-lint';
import 'jsonlint';

// Expose globals for app.js (which expects them on window or globally available)
window.CodeMirror = CodeMirror;
// window.jsonlint is set by the jsonlint import (web build)

// Import application logic
import './app.js';
