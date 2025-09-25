import express from 'express';

import config from './config.mjs'
import { convertStyleRequest, convertSourceRequest, convertTileRequest, runCLISetup } from './convert.mjs'

const app = express();

app.use('/style', convertStyleRequest);
app.use('/source', convertSourceRequest);
app.use('/tile', convertTileRequest);

app.listen(config.port, config.host, () => {
  runCLISetup();

  console.log(`Server started on port ${config.port}`);
});