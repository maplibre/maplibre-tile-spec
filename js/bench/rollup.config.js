import commonjs from '@rollup/plugin-commonjs';
import { nodeResolve } from '@rollup/plugin-node-resolve';

export default {
  input: 'dist/bench/decodeInBrowser.js',
  output: {
    dir: 'bench/output'
  },
  plugins: [
    nodeResolve(),
    commonjs()
  ]
};
