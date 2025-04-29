import ccxt
import pandas as pd
ex = ccxt.binance()
from_ts = ex.parse8601('2020-07-21 00:00:00')
ohlcv = ex.fetch_ohlcv('BTC/USDT', '5m', since=from_ts, limit=1000)
from_ts = ex.parse8601('2024-012-21 00:00:00')
ohlcv_list = []
ohlcv = ex.fetch_ohlcv('BTC/USDT', '5m', since=from_ts, limit=1000)
ohlcv_list.append(ohlcv)
while True:
    from_ts = ohlcv[-1][0]
    new_ohlcv = ex.fetch_ohlcv('BTC/USDT', '5m', since=from_ts, limit=1000)
    ohlcv.extend(new_ohlcv)
    if len(new_ohlcv)!=1000:
    	break

df = pd.DataFrame(ohlcv, columns=['date', 'open', 'high', 'low', 'close', 'volume'])
df['date'] = pd.to_datetime(df['date'], unit='ms')
df.set_index('date', inplace=True)
df = df.sort_index(ascending=True)
df.to_csv('market_data.csv', index=False)