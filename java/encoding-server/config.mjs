import yargs from 'yargs';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const configFile = JSON.parse(fs.readFileSync(path.join(__dirname, 'config.json')));

const config = yargs(process.argv.slice(2))
  .option('host', {
    type: 'string',
    default: (configFile && configFile.host) ? configFile.host : '0.0.0.0'
  })
  .option('port', {
    type: 'number',
    default: (configFile && configFile.port) ? configFile.port : 80
  })
  .option('verbose', {
    type: 'boolean',
    default: (configFile && configFile.verbose) ? configFile.verbose : true
  })
  .option('keep_files', {
    type: 'boolean',
    default: (configFile && configFile.keep_files) ? configFile.keep_files : false
  })
  .option('noencodingserver', {
    type: 'boolean',
    default: (configFile && configFile.noencodingserver) ? configFile.noencodingserver : false
  })

  // encoding params
  .option('input', {
    type: 'string',
    choices: [ 'mvt', 'pmtiles' ],
    default: (configFile && configFile.input) ? configFile.input : 'mvt'
  })
  .option('noids', {
    type: 'boolean',
    default: (configFile && configFile.noids) ? configFile.noids : false
  })
  .option('advanced', {
    type: 'boolean',
    default: (configFile && configFile.advanced) ? configFile.advanced : false
  })
  .option('nomorton', {
    type: 'boolean',
    default: (configFile && configFile.nomorton) ? configFile.nomorton : false
  })
  .option('outlines', {
    type: 'string',
    default: (configFile && configFile.outlines) ? configFile.outlines : ''
  })
  .option('timer', {
    type: 'boolean',
    default: (configFile && configFile.timer) ? configFile.timer : false
  })
  .option('compare', {
    type: 'boolean',
    default: (configFile && configFile.compare) ? configFile.compare : false
  })
  .argv;


config.cachePath = path.join(__dirname, 'cache');
config.cliToolsPath = path.join(__dirname, '../');
config.encoderPath = path.join(config.cliToolsPath, 'build/libs/encode.jar');
config.encoderPort = 3001;

export default config;
