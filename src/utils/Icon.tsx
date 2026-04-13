import {
  Search, Trash2, Settings, Smartphone, X,
  ArrowUp, Image as ImageIcon, FileText, Code,
  Star, ArrowLeft, Pin, Globe, Edit, Eye, RefreshCw, Plus, Ban, Languages
} from "lucide-react";

export const Icon = ({ name, size = 16, style = {} }: any) => {
  const props = { size, style };
  switch (name) {
    case 'search': return <Search {...props} />;
    case 'trash': return <Trash2 {...props} />;
    case 'settings': return <Settings {...props} />;
    case 'phone': return <Smartphone {...props} />;
    case 'close': return <X {...props} />;
    case 'back': return <ArrowLeft {...props} />;
    case 'arrow-up': return <ArrowUp {...props} />;
    case 'eye': return <Eye {...props} />;
    case 'edit': return <Edit {...props} />;
    case 'globe': return <Globe {...props} />;
    case 'ban': return <Ban {...props} />;
    case 'text': return <FileText {...props} />;
    case 'image': return <ImageIcon {...props} />;
    case 'code': return <Code {...props} />;
    case 'custom': return <Star {...props} />;
    case 'refresh': return <RefreshCw {...props} />;
    case 'translate-selection': return <Languages {...props} />;
    case 'plus-img': return <Plus {...props} />;
    case 'pin': return <Pin {...props} />;
    default: return null;
  }
};
