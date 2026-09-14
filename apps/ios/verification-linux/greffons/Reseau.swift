// Ce que Linux range ailleurs : `URLSession`, `URLRequest` et leurs voisines
// vivent dans FoundationNetworking et non dans Foundation. Sur les plateformes
// Apple, `canImport(FoundationNetworking)` est faux et ce fichier ne contient
// rien.
#if canImport(FoundationNetworking)
@_exported import FoundationNetworking
#endif
