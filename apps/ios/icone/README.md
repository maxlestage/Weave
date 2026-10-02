# L'icône

`icone.svg` est la source : le logo du site (`apps/web/public/favicon.svg`),
en carré plein et sans coins arrondis — iOS et watchOS appliquent eux-mêmes
leur masque. Apple refuse une icône transparente ou porteuse d'un canal alpha.

Les deux PNG de 1024 px (`Weave/Assets.xcassets` et
`WeaveWatch/Assets.xcassets`) en sont tirés avec Chromium sans tête :

```sh
printf '<!doctype html><body style="margin:0">%s' "$(cat icone.svg)" > /tmp/icone.html
chromium --headless=new --hide-scrollbars --window-size=1024,1024 \
  --screenshot=icone.png file:///tmp/icone.html
```
